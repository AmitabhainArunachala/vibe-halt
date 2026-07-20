import argparse
from .core.runner import MultiverseRunner


def main():
    parser = argparse.ArgumentParser(description="Mega Hyper Vibration Multiverse Halting Machine")
    parser.add_argument("--target", required=True, help="Path to file or diff")
    parser.add_argument("--universes", type=int, default=1000)
    parser.add_argument("--seed", type=int, default=42)
    args = parser.parse_args()

    runner = MultiverseRunner(args.target, args.universes, args.seed)
    report = runner.run()
    print(report.summary)
    print(report.to_dict())


if __name__ == "__main__":
    main()
