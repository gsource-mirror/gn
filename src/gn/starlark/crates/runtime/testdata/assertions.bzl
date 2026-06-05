def assert_eq(a, b):
    """Assert that two values are equal."""
    if a != b:
        fail("Assertion failed: {} != {}".format(a, b))

def assert_ne(a, b):
    """Assert that two values are not equal."""
    if a == b:
        fail("Assertion failed: {} == {}".format(a, b))