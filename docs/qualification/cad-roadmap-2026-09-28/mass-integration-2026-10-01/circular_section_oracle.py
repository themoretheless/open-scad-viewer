"""Independent numerical volume oracle: sphere minus an off-axis cylindrical drill.

Uses disk intersection areas and composite Simpson integration along Z.
This sampled oracle is not a certified bound or proof of B-rep validity.
"""
import json
import math


def section(z):
    radius = math.sqrt(max(0.0, 25.0 - z * z))
    drill = 1.0
    separation = math.sqrt(0.3**2 + 0.2**2)
    if radius + drill <= separation:
        return 0.0
    if separation <= abs(radius - drill):
        return math.pi * min(radius, drill)**2
    a = (separation**2 + radius**2 - drill**2) / (2 * separation * radius)
    b = (separation**2 + drill**2 - radius**2) / (2 * separation * drill)
    product = ((-separation + radius + drill) * (separation + radius - drill)
               * (separation - radius + drill) * (separation + radius + drill))
    return (radius**2 * math.acos(max(-1, min(1, a)))
            + drill**2 * math.acos(max(-1, min(1, b)))
            - 0.5 * math.sqrt(max(0, product)))


values = []
for panels in (10_000, 100_000):
    step = 10 / panels
    removed = step / 3 * (section(-5) + section(5)
        + sum((4 if i % 2 else 2) * section(-5 + i * step)
              for i in range(1, panels)))
    values.append({'panels': panels, 'volumeMm3': 4 * math.pi * 125 / 3 - removed})
print(json.dumps({'status': 'numerical_oracle', 'results': values,
    'differenceMm3': abs(values[1]['volumeMm3'] - values[0]['volumeMm3'])}, indent=2))
