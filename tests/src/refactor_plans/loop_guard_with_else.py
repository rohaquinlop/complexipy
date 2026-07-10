def sample(items, threshold):
    total = 0
    for item in items:
        if item.active:
            if item.value > threshold:
                total += item.value
            else:
                total += threshold
    return total
