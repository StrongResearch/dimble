import multiprocessing
import time


def func(xs):
    time.sleep(1)
    return [x * x for x in xs]


def main():
    xs = list(range(100))
    with multiprocessing.Pool(16) as p:
        ys = p.starmap(func, [(xs[i::16],) for i in range(16)])
    print(ys)


if __name__ == "__main__":
    main()
