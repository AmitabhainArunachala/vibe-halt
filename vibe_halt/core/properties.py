from typing import Any, Callable, Dict, List


class Property:
    """Base class for integrity properties."""

    def __init__(self, name: str, check_fn: Callable[[Any], bool]):
        self.name = name
        self.check_fn = check_fn

    def check(self, universe_result: Any) -> bool:
        return self.check_fn(universe_result)


def register_property(registry: Dict[str, Property]):
    def decorator(cls):
        prop = cls()
        registry[prop.name] = prop
        return cls
    return decorator


# Starter properties for Phase 1
PROPERTIES: Dict[str, Property] = {}


@register_property(PROPERTIES)
class NoHallucinatedAPIs(Property):
    def __init__(self):
        super().__init__(
            "NoHallucinatedAPIs",
            lambda result: not any("hallucinated" in str(e).lower() for e in getattr(result, 'exceptions', [])),
        )


@register_property(PROPERTIES)
class StateCoherent(Property):
    def __init__(self):
        super().__init__(
            "StateCoherent",
            lambda result: getattr(result, 'state_coherent', True),
        )
