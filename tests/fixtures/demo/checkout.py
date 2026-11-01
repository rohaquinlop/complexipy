def apply_discount(order, user):
    if user is not None and user.is_active:
        if order.total > 100:
            for item in order.items:
                if item.category == "books" or item.category == "music":
                    item.price *= 0.9
                elif item.on_sale and not item.clearance:
                    item.price *= 0.95
        return order
    return None


def shipping_cost(order):
    if order.country == "US":
        return 5
    elif order.country == "CA":
        return 8
    elif order.country == "MX":
        return 10
    return 20


def total(order):
    return sum(item.price for item in order.items)
