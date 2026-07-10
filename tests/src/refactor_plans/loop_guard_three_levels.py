def sample(items):
    total = 0
    for item in items:
        if item.active:
            if item.ready:
                if item.valid:
                    total += item.value
    return total
