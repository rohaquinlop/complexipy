def update_parent_rows(self, acct, srow, erow):
    ids = acct.rsplit(':', 1)
    our_id = ids[0] if len(ids) > 1 else ''
    if our_id != '':
        p_id = self.update_parent_rows(our_id, srow - 1, erow + 1)
        if our_id not in self.accounts:
            self.accounts.create_node(identifier=our_id, parent=p_id)
    return our_id
