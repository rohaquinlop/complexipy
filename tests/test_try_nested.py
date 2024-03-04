def test_try():
    a = None
    b = None
    try:
        a += 1
        b += 1
    except TypeError:  # 1 (nested = 0)
        if a is None:  # 2 (nested = 1)
            print("Caught a TypeError")
    except ValueError:  # 1 (nested = 0)
        if b is None:  # 2 (nested = 1)
            print("Caught a ValueError")
    except Exception as e:  # 1 (nested = 0)
        print(f"Caught an exception: {str(e)}")
    else:
        if a is not None and b is not None:  # 2 + 1 (nested = 1)
            print("No exception occurred")
    finally:
        if a:  # 2 (nested = 1)
            print("Always executed")
