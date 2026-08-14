export interface Repository<T> {
  find(id: string): T | undefined;
  save(value: T): void;
}

export type User = {
  id: string;
  name: string;
};

export class MemoryRepository implements Repository<User> {
  private readonly users = new Map<string, User>();

  find(id: string): User | undefined {
    return this.users.get(id);
  }

  save(user: User): void {
    this.users.set(user.id, user);
  }
}

export function displayName(user: User): string {
  return user.name;
}
