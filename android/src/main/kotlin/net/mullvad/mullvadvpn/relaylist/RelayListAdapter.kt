package net.mullvad.mullvadvpn.relaylist

import java.lang.ref.WeakReference
import java.util.LinkedList

import android.support.v7.widget.RecyclerView.Adapter
import android.view.LayoutInflater
import android.view.ViewGroup

import net.mullvad.mullvadvpn.R

class RelayListAdapter : Adapter<RelayItemHolder>() {
    private var relayList: RelayList? = null
    private var selectedItem: RelayItem? = null
    private val activeIndices = LinkedList<WeakReference<RelayListAdapterPosition>>()
    private var selectedItemHolder: RelayItemHolder? = null

    var onSelect: ((RelayItem?) -> Unit)? = null

    override fun onCreateViewHolder(parentView: ViewGroup, type: Int): RelayItemHolder {
        val inflater = LayoutInflater.from(parentView.context)
        val view = inflater.inflate(R.layout.relay_list_item, parentView, false)
        val index = RelayListAdapterPosition(0)

        activeIndices.add(WeakReference(index))

        return RelayItemHolder(view, this, index)
    }

    override fun onBindViewHolder(holder: RelayItemHolder, position: Int) {
        val relayList = this.relayList

        if (relayList != null) {
            var remaining = position

            for (country in relayList.countries) {
                val itemOrCount = country.getItem(remaining)

                when (itemOrCount) {
                    is GetItemResult.Item -> {
                        bindHolderToItem(holder, itemOrCount.item, position)
                        return
                    }
                    is GetItemResult.Count -> remaining -= itemOrCount.count
                }
            }
        }
    }

    override fun getItemCount() =
        relayList?.countries?.map { country -> country.visibleItemCount }?.sum() ?: 0

    fun onRelayListChange(relayList: RelayList, selectedItem: RelayItem?) {
        this.relayList = relayList
        this.selectedItem = selectedItem

        notifyDataSetChanged()
    }

    fun selectItem(item: RelayItem?, holder: RelayItemHolder?) {
        selectedItemHolder?.selected = false

        selectedItem = item
        selectedItemHolder = holder
        selectedItemHolder?.apply { selected = true }

        onSelect?.invoke(item)
    }

    fun expandItem(itemIndex: RelayListAdapterPosition, childCount: Int) {
        val position = itemIndex.position

        updateActiveIndices(position, childCount)
        notifyItemRangeInserted(position + 1, childCount)
    }

    fun collapseItem(itemIndex: RelayListAdapterPosition, childCount: Int) {
        val position = itemIndex.position

        updateActiveIndices(position, -childCount)
        notifyItemRangeRemoved(position + 1, childCount)
    }

    private fun updateActiveIndices(position: Int, delta: Int) {
        val activeIndicesIterator = activeIndices.iterator()

        while (activeIndicesIterator.hasNext()) {
            val index = activeIndicesIterator.next().get()

            if (index == null) {
                activeIndicesIterator.remove()
            } else {
                val indexPosition = index.position

                if (indexPosition > position) {
                    index.position = indexPosition + delta
                }
            }
        }
    }

    private fun bindHolderToItem(holder: RelayItemHolder, item: RelayItem, position: Int) {
        holder.item = item
        holder.itemPosition.position = position

        if (selectedItem != null && selectedItem == item) {
            holder.selected = true
            selectedItemHolder = holder
        } else {
            holder.selected = false
        }
    }
}
