def sample(items):
    total = 0
    for item in items:
        if item.active:
            with lock:
                total += 1
            if item.ready:
                total += 2
        total += 3
    return total
