def test_try():
    try:
        # Code that may raise an exception
        pass
    except TypeError:
        if True:
            print("Caught a TypeError")
    except ValueError:
        if True:
            print("Caught a ValueError")
    except Exception as e:
        print(f"Caught an exception: {str(e)}")
    else:
        if True:
            print("No exception occurred")
    finally:
        if True:
            print("Always executed")
