"""Mega Hyper Vibration Multiverse Halting Machine — strict Python client."""

__version__ = "0.1.0"

from .core.request import EnginePolicy, RunRequest
from .core.result import Grade, Outcome, Tier, Verdict
from .core.runner import MultiverseRunner

__all__ = [
    "EnginePolicy",
    "Grade",
    "MultiverseRunner",
    "Outcome",
    "RunRequest",
    "Tier",
    "Verdict",
]
