export {};
declare global {
  interface Array<T> {
    sortBy<T>(cfg): Array<T>;
  }
}
