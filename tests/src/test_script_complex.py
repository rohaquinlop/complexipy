lst = []

for i in range(2, 11, 2):  # +1
    if i % 3 == 0:  # +2 (nesting)
        lst.append(i + 1)
    else:  # +1
        lst.append(i)

for i in lst:  # +1
    if i > 5:  # +2 (nesting)
        print(i)
