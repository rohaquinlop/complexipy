def simple_func(x):
    if x:  # +1
        return 1
    return 0


for i in range(10):  # +1
    if i > 5:  # +2 (nesting)
        print(i)
