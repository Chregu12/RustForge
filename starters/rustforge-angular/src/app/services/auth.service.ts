import { Injectable, signal } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { Router } from '@angular/router';
import { firstValueFrom } from 'rxjs';

export interface User {
  id: number;
  name: string;
  email: string;
  email_verified_at?: string;
  created_at: string;
  updated_at: string;
}

export interface AuthResponse {
  user: User;
  token: string;
}

@Injectable({
  providedIn: 'root',
})
export class AuthService {
  private readonly apiUrl = '/api';

  user = signal<User | null>(null);
  loading = signal(true);

  constructor(
    private http: HttpClient,
    private router: Router,
  ) {
    this.checkAuth();
  }

  get isAuthenticated(): boolean {
    return !!this.user();
  }

  async checkAuth(): Promise<void> {
    const token = localStorage.getItem('token');
    if (!token) {
      this.loading.set(false);
      return;
    }

    try {
      const user = await firstValueFrom(this.http.get<User>(`${this.apiUrl}/user`));
      this.user.set(user);
    } catch {
      localStorage.removeItem('token');
    } finally {
      this.loading.set(false);
    }
  }

  async login(email: string, password: string, remember?: boolean): Promise<void> {
    const response = await firstValueFrom(
      this.http.post<AuthResponse>(`${this.apiUrl}/auth/login`, { email, password, remember })
    );
    localStorage.setItem('token', response.token);
    this.user.set(response.user);
  }

  async register(name: string, email: string, password: string, passwordConfirmation: string): Promise<void> {
    const response = await firstValueFrom(
      this.http.post<AuthResponse>(`${this.apiUrl}/auth/register`, {
        name,
        email,
        password,
        password_confirmation: passwordConfirmation,
      })
    );
    localStorage.setItem('token', response.token);
    this.user.set(response.user);
  }

  async logout(): Promise<void> {
    try {
      await firstValueFrom(this.http.post(`${this.apiUrl}/auth/logout`, {}));
    } finally {
      localStorage.removeItem('token');
      this.user.set(null);
      this.router.navigate(['/login']);
    }
  }

  async forgotPassword(email: string): Promise<{ message: string }> {
    return firstValueFrom(
      this.http.post<{ message: string }>(`${this.apiUrl}/auth/forgot-password`, { email })
    );
  }

  async resetPassword(token: string, email: string, password: string, passwordConfirmation: string): Promise<{ message: string }> {
    return firstValueFrom(
      this.http.post<{ message: string }>(`${this.apiUrl}/auth/reset-password`, {
        token,
        email,
        password,
        password_confirmation: passwordConfirmation,
      })
    );
  }

  async updateProfile(name: string, email: string): Promise<User> {
    const user = await firstValueFrom(
      this.http.put<User>(`${this.apiUrl}/user/profile`, { name, email })
    );
    this.user.set(user);
    return user;
  }

  async updatePassword(currentPassword: string, password: string, passwordConfirmation: string): Promise<{ message: string }> {
    return firstValueFrom(
      this.http.put<{ message: string }>(`${this.apiUrl}/user/password`, {
        current_password: currentPassword,
        password,
        password_confirmation: passwordConfirmation,
      })
    );
  }
}
