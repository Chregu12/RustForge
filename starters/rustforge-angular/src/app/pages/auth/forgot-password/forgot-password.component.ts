import { Component, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { RouterLink } from '@angular/router';
import { GuestLayoutComponent } from '../../../layouts/guest-layout/guest-layout.component';
import { AuthService } from '../../../services/auth.service';

@Component({
  selector: 'app-forgot-password',
  standalone: true,
  imports: [GuestLayoutComponent, FormsModule, RouterLink],
  template: `
    <app-guest-layout>
      <h2 class="text-2xl font-bold text-center text-gray-900 dark:text-white mb-4">
        Forgot your password?
      </h2>
      <p class="text-center text-sm text-gray-600 dark:text-gray-400 mb-6">
        No problem. Just let us know your email address and we will email you a password reset link.
      </p>

      <form (ngSubmit)="handleSubmit()" class="space-y-4">
        @if (error()) {
          <div class="bg-red-50 dark:bg-red-900/50 text-red-600 dark:text-red-400 p-3 rounded-md text-sm">
            {{ error() }}
          </div>
        }

        @if (status()) {
          <div class="bg-green-50 dark:bg-green-900/50 text-green-600 dark:text-green-400 p-3 rounded-md text-sm">
            {{ status() }}
          </div>
        }

        <div class="space-y-2">
          <label for="email" class="text-sm font-medium leading-none">Email</label>
          <input
            id="email"
            type="email"
            [(ngModel)]="email"
            name="email"
            placeholder="you@example.com"
            required
            autofocus
            class="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
          />
        </div>

        <button
          type="submit"
          [disabled]="loading()"
          class="w-full inline-flex items-center justify-center whitespace-nowrap rounded-md text-sm font-medium bg-primary text-primary-foreground h-10 px-4 py-2 hover:bg-primary/90 disabled:opacity-50"
        >
          {{ loading() ? 'Sending...' : 'Email Password Reset Link' }}
        </button>

        <p class="text-center text-sm text-gray-600 dark:text-gray-400">
          Remember your password?
          <a routerLink="/login" class="text-primary hover:underline">Sign in</a>
        </p>
      </form>
    </app-guest-layout>
  `,
})
export class ForgotPasswordComponent {
  email = '';
  status = signal<string | null>(null);
  error = signal<string | null>(null);
  loading = signal(false);

  constructor(private authService: AuthService) {}

  async handleSubmit() {
    this.error.set(null);
    this.status.set(null);
    this.loading.set(true);

    try {
      const response = await this.authService.forgotPassword(this.email);
      this.status.set(response.message);
    } catch (err: unknown) {
      const e = err as { error?: { message?: string } };
      this.error.set(e.error?.message || 'Failed to send reset link');
    } finally {
      this.loading.set(false);
    }
  }
}
