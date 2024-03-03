def test_try():
    try:
        # Code that may raise an exception
        pass
    except TypeError:
        print("Caught a TypeError")
    except ValueError:
        print("Caught a ValueError")
    except Exception as e:
        print(f"Caught an exception: {str(e)}")
