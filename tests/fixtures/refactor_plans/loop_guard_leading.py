def sample(items):
    total = 0
    for item in items:
        total += item.value
        if item.active:
            with lock:
                total += 1
            if item.ready:
                total += 2
    return total
