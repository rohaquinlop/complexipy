def sample(items):
    total = 0
    for item in (
        items
    ):
        if item.active:
            try:
                total += item.value
            except Exception:
                pass
    return total
