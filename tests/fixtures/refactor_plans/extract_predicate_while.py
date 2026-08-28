def wait_until(items, limit):
    i = 0
    while i < len(items) and items[i] < limit or i > 1000:
        i += 1
    return i
