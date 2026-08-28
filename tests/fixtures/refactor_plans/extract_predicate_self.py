class Account:
    def is_ready(self, limit):
        if self.balance > 0 and self.status == "active" or limit < self.balance:
            return True
        return False
