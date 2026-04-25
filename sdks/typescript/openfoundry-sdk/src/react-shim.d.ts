declare module 'react' {
  export type ReactNode = unknown;
  export type SetStateAction<S> = S | ((previousState: S) => S);
  export type Dispatch<A> = (value: A) => void;
  export interface Context<T> {
    Provider: unknown;
  }
  export function createContext<T>(defaultValue: T): Context<T>;
  export function createElement(type: unknown, props: unknown, ...children: unknown[]): unknown;
  export function useEffect(effect: () => void | (() => void), deps?: readonly unknown[]): void;
  export function useContext<T>(context: Context<T>): T;
  export function useMemo<T>(factory: () => T, deps: readonly unknown[]): T;
  export function useRef<T>(initialValue: T): { current: T };
  export function useState<S>(initialState: S | (() => S)): [S, Dispatch<SetStateAction<S>>];
}
