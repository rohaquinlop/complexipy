# def sum_values(values):
#     total = 0
#     for value in values:
#         total += value
#     return total

def sum_values_with_if(values):
    total = 0
    for value in values:
        if value > 0 and value < 10 and value % 2 == 0:
            total += value
    return total
