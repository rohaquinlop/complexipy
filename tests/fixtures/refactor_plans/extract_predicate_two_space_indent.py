def eligible(user, order):
  if user.is_active and user.has_subscription or order.total > 100 and not order.is_gift:
    return True
  return False
