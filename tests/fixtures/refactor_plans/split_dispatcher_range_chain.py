def classify(score):
    if score < 60:
        return "fail"
    elif score < 70:
        return "d"
    elif score < 85:
        return "b"
    elif score < 95:
        return "a"
    return "a+"
