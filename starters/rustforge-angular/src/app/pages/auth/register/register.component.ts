import { Component, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { Router, RouterLink } from '@angular/router';
import { GuestLayoutComponent } from '../../../layouts/guest-layout/guest-layout.component';
import { AuthService } from '../../../services/auth.service';

@Component({
  selector: 'app-register',
  standalone: true,
  imports: [GuestLayoutComponent, FormsModule, RouterLink],
  template: `
    <app-guest-layout>
      <h2 class="text-2xl font-bold text-center text-gray-900 dark:text-white mb-6">
        Create your account
      </h2>

      <form (ngSubmit)="handleSubmit()" class="space-y-4">
        @if (error()) {
          <div class="bg-red-50 dark:bg-red-900/50 text-red-600 dark:text-red-400 p-3 rounded-md text-sm">
            {{ error() }}
          </div>
        }

        <div class="space-y-2">
          <label for="name" class="text-sm font-medium leading-none">Name</label>
          <input
            id="name"
            [(ngModel)]="name"
            name="name"
            placeholder="John Doe"
            required
            autofocus
            class="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
          />
        </div>

        <div class="space-y-2">
          <label for="email" class="text-sm font-medium leading-none">Email</label>
          <input
            id="email"
            type="email"
            [(ngModel)]="email"
            name="email"
            placeholder="you@example.com"
            required
            class="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
          />
        </div>

        <div class="space-y-2">
          <label for="password" class="text-sm font-medium leading-none">Password</label>
          <input
            id="password"
            type="password"
            [(ngModel)]="password"
            name="password"
            placeholder="••••••••"
            required
            minlength="8"
            class="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
          />
        </div>

        <div class="space-y-2">
          <label for="password_confirmation" class="text-sm font-medium leading-none">Confirm Password</label>
          <input
            id="password_confirmation"
            type="password"
            [(ngModel)]="passwordConfirmation"
            name="passwordConfirmation"
            placeholder="••••••••"
            required
            class="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
          />
        </div>

        <button
          type="submit"
          [disabled]="loading()"
          class="w-full inline-flex items-center justify-center whitespace-nowrap rounded-md text-sm font-medium bg-primary text-primary-foreground h-10 px-4 py-2 hover:bg-primary/90 disabled:opacity-50"
        >
          {{ loading() ? 'Creating account...' : 'Create account' }}
        </button>

        <p class="text-center text-sm text-gray-600 dark:text-gray-400">
          Already have an account?
          <a routerLink="/login" class="text-primary hover:underline">Sign in</a>
        </p>
      </form>
    </app-guest-layout>
  `,
})
export class RegisterComponent {
  name = '';
  email = '';
  password = '';
  passwordConfirmation = '';
  error = signal<string | null>(null);
  loading = signal(false);

  constructor(
    private authService: AuthService,
    private router: Router,
  ) {}

  async handleSubmit() {
    this.error.set(null);
    this.loading.set(true);

    try {
      await this.authService.register(this.name, this.email, this.password, this.passwordConfirmation);
      this.router.navigate(['/dashboard']);
    } catch (err: unknown) {
      const e = err as { error?: { message?: string; errors?: Record<string, string[]> } };
      if (e.error?.errors) {
        const firstError = Object.values(e.error.errors)[0];
        this.error.set(Array.isArray(firstError) ? firstError[0] : String(firstError));
      } else {
        this.error.set(e.error?.message || 'Registration failed');
      }
    } finally {
      this.loading.set(false);
    }
  }
}
