use super::{FirewallArguments, FirewallPolicy, FirewallT};
use crate::tunnel;
use ipnetwork::IpNetwork;
use lazy_static::lazy_static;
use libc;
use nftnl::{
    self,
    expr::{self, Payload, Verdict},
    nft_expr, table, Batch, Chain, FinalizedBatch, ProtoFamily, Rule, Table,
};
use std::{
    env,
    ffi::{CStr, CString},
    io,
    net::{IpAddr, Ipv4Addr},
};
use talpid_types::net::{Endpoint, TransportProtocol};

pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can happen when interacting with Linux netfilter.
#[derive(err_derive::Error, Debug)]
pub enum Error {
    /// Unable to open netlink socket to netfilter.
    #[error(display = "Unable to open netlink socket to netfilter")]
    NetlinkOpenError(#[error(cause)] io::Error),

    /// Unable to send netlink command to netfilter.
    #[error(display = "Unable to send netlink command to netfilter")]
    NetlinkSendError(#[error(cause)] io::Error),

    /// Error while reading from netlink socket.
    #[error(display = "Error while reading from netlink socket")]
    NetlinkRecvError(#[error(cause)] io::Error),

    /// Error while processing an incoming netlink message.
    #[error(display = "Error while processing an incoming netlink message")]
    ProcessNetlinkError(#[error(cause)] io::Error),

    /// Failed to verify that our tables are set. Probably means that
    /// it's the host that does not support nftables properly.
    #[error(display = "Failed to set firewall rules")]
    NetfilterTableNotSetError,

    /// Unable to translate network interface name into index.
    #[error(
        display = "Unable to translate network interface name \"{}\" into index",
        _0
    )]
    LookupIfaceIndexError(String, #[error(cause)] crate::linux::IfaceIndexLookupError),
}

lazy_static! {
    /// TODO(linus): This crate is not supposed to be Mullvad-aware. So at some point this should be
    /// replaced by allowing the table name to be configured from the public API of this crate.
    static ref TABLE_NAME: CString = CString::new("mullvad").unwrap();
    static ref IN_CHAIN_NAME: CString = CString::new("in").unwrap();
    static ref OUT_CHAIN_NAME: CString = CString::new("out").unwrap();

    /// Allows controlling whether firewall rules should have packet counters or not from an env
    /// variable. Useful for debugging the rules.
    static ref ADD_COUNTERS: bool = env::var("TALPID_FIREWALL_DEBUG")
        .map(|v| v == "1")
        .unwrap_or(false);
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
enum Direction {
    In,
    Out,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
enum End {
    Src,
    Dst,
}

/// The Linux implementation for the firewall and DNS.
pub struct Firewall {
    table_name: CString,
}

impl FirewallT for Firewall {
    type Error = Error;

    fn new(_args: FirewallArguments) -> Result<Self> {
        Ok(Firewall {
            table_name: TABLE_NAME.clone(),
        })
    }

    fn apply_policy(&mut self, policy: FirewallPolicy) -> Result<()> {
        let table = Table::new(&self.table_name, ProtoFamily::Inet);
        let batch = PolicyBatch::new(&table).finalize(&policy)?;
        self.send_and_process(&batch)?;
        self.verify_tables(&[&TABLE_NAME])
    }

    fn reset_policy(&mut self) -> Result<()> {
        let table = Table::new(&self.table_name, ProtoFamily::Inet);
        let batch = {
            let mut batch = Batch::new();
            // Our batch will add and remove the table even though the goal is just to remove it.
            // This because only removing it throws a strange error if the table does not exist.
            batch.add(&table, nftnl::MsgType::Add);
            batch.add(&table, nftnl::MsgType::Del);
            batch.finalize()
        };

        log::debug!("Removing table and chain from netfilter");
        self.send_and_process(&batch)
    }
}

impl Firewall {
    fn send_and_process(&self, batch: &FinalizedBatch) -> Result<()> {
        let socket = mnl::Socket::new(mnl::Bus::Netfilter).map_err(Error::NetlinkOpenError)?;
        socket.send_all(batch).map_err(Error::NetlinkSendError)?;

        let portid = socket.portid();
        let mut buffer = vec![0; nftnl::nft_nlmsg_maxsize() as usize];

        let seq = 0;
        while let Some(message) = Self::socket_recv(&socket, &mut buffer[..])? {
            match mnl::cb_run(message, seq, portid).map_err(Error::ProcessNetlinkError)? {
                mnl::CbResult::Stop => {
                    log::trace!("cb_run STOP");
                    break;
                }
                mnl::CbResult::Ok => log::trace!("cb_run OK"),
            };
        }
        Ok(())
    }

    fn verify_tables(&self, expected_tables: &[&CStr]) -> Result<()> {
        let socket = mnl::Socket::new(mnl::Bus::Netfilter).map_err(Error::NetlinkOpenError)?;
        let portid = socket.portid();
        let seq = 0;

        let get_tables_msg = table::get_tables_nlmsg(seq);;
        socket
            .send(&get_tables_msg)
            .map_err(Error::NetlinkSendError)?;

        let mut table_set = ::std::collections::HashSet::new();
        let mut msg_buffer = vec![0; nftnl::nft_nlmsg_maxsize() as usize];

        while let Some(message) = Self::socket_recv(&socket, &mut msg_buffer)? {
            match mnl::cb_run2(message, seq, portid, table::get_tables_cb, &mut table_set)
                .map_err(Error::ProcessNetlinkError)?
            {
                mnl::CbResult::Stop => {
                    log::trace!("cb_run STOP");
                    break;
                }
                mnl::CbResult::Ok => log::trace!("cb_run OK"),
            }
        }

        for expected_table in expected_tables {
            if !table_set.contains(*expected_table) {
                log::error!(
                    "Expected '{}' netfilter table to be set, but it is not",
                    expected_table.to_string_lossy()
                );
                return Err(Error::NetfilterTableNotSetError);
            }
        }
        Ok(())
    }

    fn socket_recv<'a>(socket: &mnl::Socket, buf: &'a mut [u8]) -> Result<Option<&'a [u8]>> {
        let ret = socket.recv(buf).map_err(Error::NetlinkRecvError)?;
        log::trace!("Read {} bytes from netlink", ret);
        if ret > 0 {
            Ok(Some(&buf[..ret]))
        } else {
            Ok(None)
        }
    }
}

struct PolicyBatch<'a> {
    batch: Batch,
    in_chain: Chain<'a>,
    out_chain: Chain<'a>,
}

impl<'a> PolicyBatch<'a> {
    /// Bootstrap a new nftnl message batch object and add the initial messages creating the
    /// table and chains.
    pub fn new(table: &'a Table) -> Self {
        let mut batch = Batch::new();
        let mut out_chain = Chain::new(&*OUT_CHAIN_NAME, table);
        let mut in_chain = Chain::new(&*IN_CHAIN_NAME, table);
        out_chain.set_hook(nftnl::Hook::Out, 0);
        in_chain.set_hook(nftnl::Hook::In, 0);
        out_chain.set_policy(nftnl::Policy::Drop);
        in_chain.set_policy(nftnl::Policy::Drop);

        // A little dance that will make sure the table exists, but is cleared.
        batch.add(table, nftnl::MsgType::Add);
        batch.add(table, nftnl::MsgType::Del);
        batch.add(table, nftnl::MsgType::Add);
        batch.add(&out_chain, nftnl::MsgType::Add);
        batch.add(&in_chain, nftnl::MsgType::Add);

        PolicyBatch {
            batch,
            in_chain,
            out_chain,
        }
    }

    /// Finalize the nftnl message batch by adding every firewall rule needed to satisfy the given
    /// policy.
    pub fn finalize(mut self, policy: &FirewallPolicy) -> Result<FinalizedBatch> {
        self.add_loopback_rules()?;
        self.add_dhcp_rules();
        self.add_policy_specific_rules(policy)?;

        Ok(self.batch.finalize())
    }

    fn add_loopback_rules(&mut self) -> Result<()> {
        const LOOPBACK_IFACE_NAME: &str = "lo";
        self.batch.add(
            &allow_interface_rule(&self.out_chain, Direction::Out, LOOPBACK_IFACE_NAME)?,
            nftnl::MsgType::Add,
        );
        self.batch.add(
            &allow_interface_rule(&self.in_chain, Direction::In, LOOPBACK_IFACE_NAME)?,
            nftnl::MsgType::Add,
        );
        Ok(())
    }

    fn add_dhcp_rules(&mut self) {
        use self::TransportProtocol::Udp;
        {
            let mut out_v4 = Rule::new(&self.out_chain);
            check_port(&mut out_v4, Udp, End::Src, super::DHCPV4_CLIENT_PORT);
            check_ip(&mut out_v4, End::Dst, IpAddr::V4(Ipv4Addr::BROADCAST));
            check_port(&mut out_v4, Udp, End::Dst, super::DHCPV4_SERVER_PORT);
            add_verdict(&mut out_v4, &Verdict::Accept);
            self.batch.add(&out_v4, nftnl::MsgType::Add);
        }
        {
            let mut in_v4 = Rule::new(&self.in_chain);
            check_port(&mut in_v4, Udp, End::Src, super::DHCPV4_SERVER_PORT);
            check_port(&mut in_v4, Udp, End::Dst, super::DHCPV4_CLIENT_PORT);
            add_verdict(&mut in_v4, &Verdict::Accept);
            self.batch.add(&in_v4, nftnl::MsgType::Add);
        }
        for dhcpv6_server in &*super::DHCPV6_SERVER_ADDRS {
            let mut out_v6 = Rule::new(&self.out_chain);
            check_net(&mut out_v6, End::Src, *super::IPV6_LINK_LOCAL);
            check_port(&mut out_v6, Udp, End::Src, super::DHCPV6_CLIENT_PORT);
            check_ip(&mut out_v6, End::Dst, *dhcpv6_server);
            check_port(&mut out_v6, Udp, End::Dst, super::DHCPV6_SERVER_PORT);
            add_verdict(&mut out_v6, &Verdict::Accept);
            self.batch.add(&out_v6, nftnl::MsgType::Add);
        }
        {
            let mut in_v6 = Rule::new(&self.in_chain);
            check_net(&mut in_v6, End::Src, *super::IPV6_LINK_LOCAL);
            check_port(&mut in_v6, Udp, End::Src, super::DHCPV6_SERVER_PORT);
            check_net(&mut in_v6, End::Dst, *super::IPV6_LINK_LOCAL);
            check_port(&mut in_v6, Udp, End::Dst, super::DHCPV6_CLIENT_PORT);
            add_verdict(&mut in_v6, &Verdict::Accept);
            self.batch.add(&in_v6, nftnl::MsgType::Add);
        }
        // Outgoing Router solicitation (part of NDP)
        {
            let mut rule = Rule::new(&self.out_chain);
            check_ip(
                &mut rule,
                End::Dst,
                *super::ROUTER_SOLICITATION_OUT_DST_ADDR,
            );

            rule.add_expr(&nft_expr!(meta l4proto));
            rule.add_expr(&nft_expr!(cmp == libc::IPPROTO_ICMPV6 as u8));

            rule.add_expr(&Payload::Transport(
                nftnl::expr::TransportHeaderField::Icmpv6(nftnl::expr::Icmpv6HeaderField::Type),
            ));
            rule.add_expr(&nft_expr!(cmp == 133u8));
            rule.add_expr(&nftnl::expr::Payload::Transport(
                nftnl::expr::TransportHeaderField::Icmpv6(nftnl::expr::Icmpv6HeaderField::Code),
            ));
            rule.add_expr(&nft_expr!(cmp == 0u8));

            add_verdict(&mut rule, &Verdict::Accept);
            self.batch.add(&rule, nftnl::MsgType::Add);
        }
        // Incoming Router advertisement (part of NDP)
        {
            let mut rule = Rule::new(&self.in_chain);
            check_net(&mut rule, End::Src, *super::IPV6_LINK_LOCAL);

            rule.add_expr(&nft_expr!(meta l4proto));
            rule.add_expr(&nft_expr!(cmp == libc::IPPROTO_ICMPV6 as u8));

            rule.add_expr(&Payload::Transport(
                nftnl::expr::TransportHeaderField::Icmpv6(nftnl::expr::Icmpv6HeaderField::Type),
            ));
            rule.add_expr(&nft_expr!(cmp == 134u8));
            rule.add_expr(&nftnl::expr::Payload::Transport(
                nftnl::expr::TransportHeaderField::Icmpv6(nftnl::expr::Icmpv6HeaderField::Code),
            ));
            rule.add_expr(&nft_expr!(cmp == 0u8));

            add_verdict(&mut rule, &Verdict::Accept);
            self.batch.add(&rule, nftnl::MsgType::Add);
        }
        // Incoming Redirect (part of NDP)
        {
            let mut rule = Rule::new(&self.in_chain);
            check_net(&mut rule, End::Src, *super::IPV6_LINK_LOCAL);

            rule.add_expr(&nft_expr!(meta l4proto));
            rule.add_expr(&nft_expr!(cmp == libc::IPPROTO_ICMPV6 as u8));

            rule.add_expr(&Payload::Transport(
                nftnl::expr::TransportHeaderField::Icmpv6(nftnl::expr::Icmpv6HeaderField::Type),
            ));
            rule.add_expr(&nft_expr!(cmp == 137u8));
            rule.add_expr(&nftnl::expr::Payload::Transport(
                nftnl::expr::TransportHeaderField::Icmpv6(nftnl::expr::Icmpv6HeaderField::Code),
            ));
            rule.add_expr(&nft_expr!(cmp == 0u8));

            add_verdict(&mut rule, &Verdict::Accept);
            self.batch.add(&rule, nftnl::MsgType::Add);
        }
    }

    fn add_policy_specific_rules(&mut self, policy: &FirewallPolicy) -> Result<()> {
        let allow_lan = match policy {
            FirewallPolicy::Connecting {
                peer_endpoint,
                pingable_hosts,
                allow_lan,
            } => {
                self.add_allow_icmp_pingable_hosts(&pingable_hosts);
                self.add_allow_endpoint_rules(peer_endpoint);
                *allow_lan
            }
            FirewallPolicy::Connected {
                peer_endpoint,
                tunnel,
                allow_lan,
            } => {
                self.add_allow_endpoint_rules(peer_endpoint);
                self.add_dns_rule(tunnel, TransportProtocol::Udp)?;
                self.add_dns_rule(tunnel, TransportProtocol::Tcp)?;
                self.add_allow_tunnel_rules(tunnel)?;
                *allow_lan
            }
            FirewallPolicy::Blocked { allow_lan } => *allow_lan,
        };

        if allow_lan {
            self.add_allow_lan_rules();
        }
        Ok(())
    }

    fn add_allow_endpoint_rules(&mut self, endpoint: &Endpoint) {
        let mut in_rule = Rule::new(&self.in_chain);
        check_endpoint(&mut in_rule, End::Src, endpoint);

        in_rule.add_expr(&nft_expr!(ct state));
        let allowed_states = nftnl::expr::ct::States::ESTABLISHED.bits();
        in_rule.add_expr(&nft_expr!(bitwise mask allowed_states, xor 0u32));
        in_rule.add_expr(&nft_expr!(cmp != 0u32));
        add_verdict(&mut in_rule, &Verdict::Accept);

        self.batch.add(&in_rule, nftnl::MsgType::Add);


        let mut out_rule = Rule::new(&self.out_chain);
        check_endpoint(&mut out_rule, End::Dst, endpoint);
        add_verdict(&mut out_rule, &Verdict::Accept);

        self.batch.add(&out_rule, nftnl::MsgType::Add);
    }

    fn add_allow_icmp_pingable_hosts(&mut self, pingable_hosts: &[IpAddr]) {
        for host in pingable_hosts {
            let icmp_proto = match &host {
                &IpAddr::V4(_) => libc::IPPROTO_ICMP as u8,
                &IpAddr::V6(_) => libc::IPPROTO_ICMPV6 as u8,
            };

            let mut out_rule = Rule::new(&self.out_chain);
            check_ip(&mut out_rule, End::Dst, *host);
            out_rule.add_expr(&nft_expr!(meta l4proto));
            out_rule.add_expr(&nft_expr!(cmp == icmp_proto));
            add_verdict(&mut out_rule, &Verdict::Accept);
            self.batch.add(&out_rule, nftnl::MsgType::Add);

            let mut in_rule = Rule::new(&self.in_chain);
            check_ip(&mut in_rule, End::Src, *host);
            in_rule.add_expr(&nft_expr!(meta l4proto));
            in_rule.add_expr(&nft_expr!(cmp == icmp_proto));
            add_verdict(&mut in_rule, &Verdict::Accept);
            self.batch.add(&in_rule, nftnl::MsgType::Add);
        }
    }

    fn add_dns_rule(
        &mut self,
        tunnel: &tunnel::TunnelMetadata,
        protocol: TransportProtocol,
    ) -> Result<()> {
        // allow DNS traffic to the tunnel gateway
        self.add_allow_dns_rule(&tunnel.interface, protocol, tunnel.ipv4_gateway.into())?;
        if let Some(ipv6_gateway) = tunnel.ipv6_gateway {
            self.add_allow_dns_rule(&tunnel.interface, protocol, ipv6_gateway.into())?;
        };
        let mut block_rule = Rule::new(&self.out_chain);
        check_port(&mut block_rule, protocol, End::Dst, 53);
        add_verdict(&mut block_rule, &Verdict::Drop);
        self.batch.add(&block_rule, nftnl::MsgType::Add);

        Ok(())
    }

    fn add_allow_dns_rule(
        &mut self,
        interface: &str,
        protocol: TransportProtocol,
        host: IpAddr,
    ) -> Result<()> {
        let mut allow_rule = Rule::new(&self.out_chain);
        let daddr = match host {
            IpAddr::V4(_) => nft_expr!(payload ipv4 daddr),
            IpAddr::V6(_) => nft_expr!(payload ipv6 daddr),
        };

        check_iface(&mut allow_rule, Direction::Out, interface)?;
        check_port(&mut allow_rule, protocol, End::Dst, 53);
        check_l3proto(&mut allow_rule, host);

        allow_rule.add_expr(&daddr);
        allow_rule.add_expr(&nft_expr!(cmp == host));
        add_verdict(&mut allow_rule, &Verdict::Accept);

        self.batch.add(&allow_rule, nftnl::MsgType::Add);
        Ok(())
    }

    fn add_allow_tunnel_rules(&mut self, tunnel: &tunnel::TunnelMetadata) -> Result<()> {
        self.batch.add(
            &allow_interface_rule(&self.out_chain, Direction::Out, &tunnel.interface[..])?,
            nftnl::MsgType::Add,
        );
        self.batch.add(
            &allow_interface_rule(&self.in_chain, Direction::In, &tunnel.interface[..])?,
            nftnl::MsgType::Add,
        );
        Ok(())
    }

    fn add_allow_lan_rules(&mut self) {
        // LAN -> LAN
        for net in &*super::ALLOWED_LAN_NETS {
            let mut out_rule = Rule::new(&self.out_chain);
            check_net(&mut out_rule, End::Dst, *net);
            add_verdict(&mut out_rule, &Verdict::Accept);
            self.batch.add(&out_rule, nftnl::MsgType::Add);

            let mut in_rule = Rule::new(&self.in_chain);
            check_net(&mut in_rule, End::Src, *net);
            add_verdict(&mut in_rule, &Verdict::Accept);
            self.batch.add(&in_rule, nftnl::MsgType::Add);
        }
        // LAN -> Multicast
        for net in &*super::ALLOWED_LAN_MULTICAST_NETS {
            let mut rule = Rule::new(&self.out_chain);
            check_net(&mut rule, End::Dst, *net);
            add_verdict(&mut rule, &Verdict::Accept);
            self.batch.add(&rule, nftnl::MsgType::Add);
        }
    }
}

fn allow_interface_rule<'a>(
    chain: &'a Chain<'_>,
    direction: Direction,
    iface: &str,
) -> Result<Rule<'a>> {
    let mut rule = Rule::new(&chain);
    check_iface(&mut rule, direction, iface)?;
    add_verdict(&mut rule, &Verdict::Accept);

    Ok(rule)
}

fn check_iface(rule: &mut Rule<'_>, direction: Direction, iface: &str) -> Result<()> {
    let iface_index = crate::linux::iface_index(iface)
        .map_err(|e| Error::LookupIfaceIndexError(iface.to_owned(), e))?;
    rule.add_expr(&match direction {
        Direction::In => nft_expr!(meta iif),
        Direction::Out => nft_expr!(meta oif),
    });
    rule.add_expr(&nft_expr!(cmp == iface_index));
    Ok(())
}

fn check_net(rule: &mut Rule<'_>, end: End, net: impl Into<IpNetwork>) {
    let net = net.into();
    // Must check network layer protocol before loading network layer payload
    check_l3proto(rule, net.ip());

    rule.add_expr(&match (net, end) {
        (IpNetwork::V4(_), End::Src) => nft_expr!(payload ipv4 saddr),
        (IpNetwork::V4(_), End::Dst) => nft_expr!(payload ipv4 daddr),
        (IpNetwork::V6(_), End::Src) => nft_expr!(payload ipv6 saddr),
        (IpNetwork::V6(_), End::Dst) => nft_expr!(payload ipv6 daddr),
    });
    match net {
        IpNetwork::V4(_) => rule.add_expr(&nft_expr!(bitwise mask net.mask(), xor 0u32)),
        IpNetwork::V6(_) => rule.add_expr(&nft_expr!(bitwise mask net.mask(), xor &[0u16; 8][..])),
    };
    rule.add_expr(&nft_expr!(cmp == net.ip()));
}

fn check_endpoint(rule: &mut Rule<'_>, end: End, endpoint: &Endpoint) {
    check_ip(rule, end, endpoint.address.ip());
    check_port(rule, endpoint.protocol, end, endpoint.address.port());
}

fn check_ip(rule: &mut Rule<'_>, end: End, ip: impl Into<IpAddr>) {
    let ip = ip.into();
    // Must check network layer protocol before loading network layer payload
    check_l3proto(rule, ip);

    rule.add_expr(&match (ip, end) {
        (IpAddr::V4(..), End::Src) => nft_expr!(payload ipv4 saddr),
        (IpAddr::V4(..), End::Dst) => nft_expr!(payload ipv4 daddr),
        (IpAddr::V6(..), End::Src) => nft_expr!(payload ipv6 saddr),
        (IpAddr::V6(..), End::Dst) => nft_expr!(payload ipv6 daddr),
    });
    match ip {
        IpAddr::V4(addr) => rule.add_expr(&nft_expr!(cmp == addr)),
        IpAddr::V6(addr) => rule.add_expr(&nft_expr!(cmp == addr)),
    }
}

fn check_port(rule: &mut Rule<'_>, protocol: TransportProtocol, end: End, port: u16) {
    // Must check transport layer protocol before loading transport layer payload
    check_l4proto(rule, protocol);

    rule.add_expr(&match (protocol, end) {
        (TransportProtocol::Udp, End::Src) => nft_expr!(payload udp sport),
        (TransportProtocol::Udp, End::Dst) => nft_expr!(payload udp dport),
        (TransportProtocol::Tcp, End::Src) => nft_expr!(payload tcp sport),
        (TransportProtocol::Tcp, End::Dst) => nft_expr!(payload tcp dport),
    });
    rule.add_expr(&nft_expr!(cmp == port.to_be()));
}

fn check_l3proto(rule: &mut Rule<'_>, ip: IpAddr) {
    rule.add_expr(&nft_expr!(meta nfproto));
    rule.add_expr(&nft_expr!(cmp == l3proto(ip)));
}

fn l3proto(addr: IpAddr) -> u8 {
    match addr {
        IpAddr::V4(_) => libc::NFPROTO_IPV4 as u8,
        IpAddr::V6(_) => libc::NFPROTO_IPV6 as u8,
    }
}

fn check_l4proto(rule: &mut Rule<'_>, protocol: TransportProtocol) {
    rule.add_expr(&nft_expr!(meta l4proto));
    rule.add_expr(&nft_expr!(cmp == l4proto(protocol)));
}

fn l4proto(protocol: TransportProtocol) -> u8 {
    match protocol {
        TransportProtocol::Udp => libc::IPPROTO_UDP as u8,
        TransportProtocol::Tcp => libc::IPPROTO_TCP as u8,
    }
}

fn add_verdict(rule: &mut Rule<'_>, verdict: &expr::Verdict) {
    if *ADD_COUNTERS {
        rule.add_expr(&nft_expr!(counter));
    }
    rule.add_expr(verdict);
}
