use std::result::Result as StdResult;

use jsonrpc_core::{Error as JsonRpcError, ErrorCode};

macro_rules! remote_errors {
    ( $( $code:expr => $kind:ident : $description:expr ),* $(,)* ) => {
        error_chain! {
            errors {
                $( $kind { description($description) } )*
            }
        }

        impl ErrorKind {
            pub fn try_from(jsonrpc_error: JsonRpcError) -> StdResult<Self, JsonRpcError> {
                match jsonrpc_error.code {
                    $( ErrorCode::ServerError($code) => Ok(ErrorKind::$kind), )*
                    _ => Err(jsonrpc_error),
                }
            }

            pub fn error_code(&self) -> Option<i32> {
                match *self {
                    $( ErrorKind::$kind => Some($code), )*
                    _ => None,
                }
            }
        }
    }
}

remote_errors! {
    -100 => InternalError: "Internal error",
    -200 => AccountDoesNotExist: "Account does not exist",
    -300 => MaxNumberOfPorts: "Maximum number of ports reached",
    -301 => PortNotForwarded: "Port not forwarded for this account",
    -400 => BadVoucher: "Invalid voucher code",
    -401 => VoucherAlreadyUsed: "Voucher code already used",
    -500 => PaymentDeclined: "The payment was declined",
    -501 => PaymentProcessorError: "An error occured while processing the payment",
    -600 => NoPaymentToAddress: "No payments are registered with that address",
    -601 => InvalidSignature: "Invalid signature",
    -602 => NoTransactionFound: "No transaction found with given input and output",
    -603 => MultipleAccountsPaidForWithAddress: "Multiple accounts paid for with that address",
    -604 => InvalidBitcoinAddress: "Invalid Bitcoin address",
}

impl Error {
    pub fn try_from(jsonrpc_error: JsonRpcError) -> StdResult<Self, JsonRpcError> {
        ErrorKind::try_from(jsonrpc_error).map(Error::from)
    }

    pub fn error_code(&self) -> Option<i32> {
        self.kind().error_code()
    }
}
