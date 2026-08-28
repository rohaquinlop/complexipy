def classify(x, y):
    if x > 10:
        return "big"
    elif x > 5 and y < 3 or x == 0:
        return "mid"
    return "small"
