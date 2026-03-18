import { helper } from "./dep";

export interface User {
  id: string;
  name: string;
}

export type UserSummary = Pick<User, "id" | "name">;

export function foo(value: number): number {
  const base = helper(value);
  return base + 1;
}

export const formatUser = (user: User): string => {
  return `${user.id}:${user.name}`;
};

export class Greeter implements User {
  id: string;
  name: string;

  constructor(id: string, name: string) {
    this.id = id;
    this.name = name;
  }

  greet(prefix: string): string {
    return `${prefix} ${this.name}`;
  }

  static create(name: string): Greeter {
    return new Greeter("generated", name);
  }
}
