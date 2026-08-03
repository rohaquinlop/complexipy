def sample(kind, data):
    match kind:
        case "a":
            return 1
        case "b":
            return 2
        case "c":
            return 3
        case "d":
            if data:
                return 4
            return 0
    return 0
