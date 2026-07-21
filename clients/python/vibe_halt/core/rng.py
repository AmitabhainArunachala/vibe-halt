import random
from typing import Any, Sequence


class SeededRNG:
    """Fully deterministic and replayable RNG for multiverse runs."""

    def __init__(self, seed: int):
        self.seed = seed
        self._rng = random.Random(seed)

    def random(self) -> float:
        return self._rng.random()

    def choice(self, seq: Sequence[Any]) -> Any:
        return self._rng.choice(seq)

    def randint(self, a: int, b: int) -> int:
        return self._rng.randint(a, b)

    def get_state(self) -> Any:
        return self._rng.getstate()

    def set_state(self, state: Any) -> None:
        self._rng.setstate(state)
