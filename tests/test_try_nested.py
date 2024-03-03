def test_try():
    a = None
    b = None
    try:
        a += 1
        b += 1
    except TypeError:
        if a is None:
            print("Caught a TypeError")
    except ValueError:
        if b is None:
            print("Caught a ValueError")
    except Exception as e:
        print(f"Caught an exception: {str(e)}")
    else:
        if a is not None and b is not None:
            print("No exception occurred")
    finally:
        if a:
            print("Always executed")
