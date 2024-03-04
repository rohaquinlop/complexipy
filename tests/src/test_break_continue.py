def function():
    for i in range(10):  # 1
        if i == 5:  # 2 (nested = 1)
            break  # 0 (nested = 2)
        continue  # 0 (nested = 1)
        print(i)
