def sample(a, b, items):
    if a:  # gate: primary
        if b:
            return items[0]
    return None
