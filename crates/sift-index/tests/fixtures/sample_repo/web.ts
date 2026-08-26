interface Options {
    retries: number;
}

enum Mode { Fast, Slow }

export function retry<T>(op: () => T): T {
    return op();
}

class RetryClient implements Options {
    retries = 3;

    fetch(): void {}
}
