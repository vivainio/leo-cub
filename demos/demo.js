export class Counter {
  constructor(initial = 0) {
    this.value = initial;
  }

  increment() {
    this.value += 1;
    return this.value;
  }
}

export function formatCount(counter) {
  return `Count: ${counter.value}`;
}

export function* countFrom(start) {
  let value = start;
  while (true) {
    yield value++;
  }
}
