def update_parent_rows(self, acct, srow, erow) -> str:
    ids = acct.rsplit(':', 1)
    our_id = ids[0] if len(ids) > 1 else ''
    if our_id != '':
        p_id = self.update_parent_rows(our_id, srow - 1, erow + 1)
        if our_id not in self.accounts:
            self.accounts.create_node(
                identifier=our_id,
                parent=p_id,
                data=self.TrsAcct(srow, self.periods_reported),
            )
    our_node = self.accounts[our_id]
    our_node.data.start_row = min(our_node.data.start_row, srow)
    our_node.data.end_row = max(our_node.data.end_row, erow)
    return our_id
