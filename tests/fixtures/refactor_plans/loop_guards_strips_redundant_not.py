def process(items):
    results = []
    for item in items:
        if not item.active:
            if item.ready:
                results.append(item)
            else:
                results.append(None)
    return results
