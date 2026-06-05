initial = 0

def sum(*args, **kwargs):
  # Should be able to access module scoped variables.
  total = initial
  for arg in args:
    total += arg
  for _, v in kwargs.items():
    total += v
  return total

pInfo = provider(fields = {'foo': 'bar'})

s = struct(
  b = True,
  n = 1,
  s = 'hello',
  l = [1, 'hello'],
  d = dict(
    n = 1,
  ),
  p = pInfo(foo = 1),
)

def complex_input_type(d):
  return d['n']