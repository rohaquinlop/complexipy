def sum_values(values):
    total = 0
    for value in values:  # 1 (nested = 0)
        total += value
    return total


def sum_values_with_if(values):
    total = 0
    for value in values:  # 1 (nested = 0)
        if value > 0 and value < 10 and value % 2 == 0:  # [2] + 1 (nested = 1)
            total += value
    return total
