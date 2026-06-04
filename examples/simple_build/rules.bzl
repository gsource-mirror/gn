b = True
n = 1
s = "hello"
l = [n, s]
d = dict(
  n = 1
)

def f(*args, **kwargs):
  return (args, kwargs, s)
