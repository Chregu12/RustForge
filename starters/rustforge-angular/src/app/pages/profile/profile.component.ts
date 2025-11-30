import { Component, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { AppLayoutComponent } from '../../layouts/app-layout/app-layout.component';
import { AuthService } from '../../services/auth.service';

@Component({
  selector: 'app-profile',
  standalone: true,
  imports: [AppLayoutComponent, FormsModule],
  template: `
    <app-app-layout>
      <div class="space-y-6">
        <!-- Profile Information Card -->
        <div class="rounded-lg border bg-card text-card-foreground shadow-sm">
          <div class="flex flex-col space-y-1.5 p-6">
            <h3 class="text-2xl font-semibold leading-none tracking-tight">Profile Information</h3>
            <p class="text-sm text-muted-foreground">Update your account's profile information and email address.</p>
          </div>
          <div class="p-6 pt-0">
            <form (ngSubmit)="updateProfile()" class="space-y-4">
              <div class="space-y-2">
                <label for="name" class="text-sm font-medium leading-none">Name</label>
                <input
                  id="name"
                  [(ngModel)]="profileData.name"
                  name="name"
                  required
                  class="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
                />
              </div>

              <div class="space-y-2">
                <label for="email" class="text-sm font-medium leading-none">Email</label>
                <input
                  id="email"
                  type="email"
                  [(ngModel)]="profileData.email"
                  name="email"
                  required
                  class="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
                />
              </div>

              @if (profileStatus()) {
                <p [class]="profileStatus()?.includes('success') ? 'text-sm text-green-600' : 'text-sm text-red-600'">
                  {{ profileStatus() }}
                </p>
              }

              <button
                type="submit"
                [disabled]="profileLoading()"
                class="inline-flex items-center justify-center whitespace-nowrap rounded-md text-sm font-medium bg-primary text-primary-foreground h-10 px-4 py-2 hover:bg-primary/90 disabled:opacity-50"
              >
                {{ profileLoading() ? 'Saving...' : 'Save' }}
              </button>
            </form>
          </div>
        </div>

        <!-- Update Password Card -->
        <div class="rounded-lg border bg-card text-card-foreground shadow-sm">
          <div class="flex flex-col space-y-1.5 p-6">
            <h3 class="text-2xl font-semibold leading-none tracking-tight">Update Password</h3>
            <p class="text-sm text-muted-foreground">Ensure your account is using a long, random password to stay secure.</p>
          </div>
          <div class="p-6 pt-0">
            <form (ngSubmit)="updatePassword()" class="space-y-4">
              <div class="space-y-2">
                <label for="current_password" class="text-sm font-medium leading-none">Current Password</label>
                <input
                  id="current_password"
                  type="password"
                  [(ngModel)]="passwordData.currentPassword"
                  name="currentPassword"
                  required
                  class="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
                />
              </div>

              <div class="space-y-2">
                <label for="password" class="text-sm font-medium leading-none">New Password</label>
                <input
                  id="password"
                  type="password"
                  [(ngModel)]="passwordData.password"
                  name="password"
                  required
                  class="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
                />
              </div>

              <div class="space-y-2">
                <label for="password_confirmation" class="text-sm font-medium leading-none">Confirm Password</label>
                <input
                  id="password_confirmation"
                  type="password"
                  [(ngModel)]="passwordData.passwordConfirmation"
                  name="passwordConfirmation"
                  required
                  class="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
                />
              </div>

              @if (passwordStatus()) {
                <p [class]="passwordStatus()?.includes('success') ? 'text-sm text-green-600' : 'text-sm text-red-600'">
                  {{ passwordStatus() }}
                </p>
              }

              <button
                type="submit"
                [disabled]="passwordLoading()"
                class="inline-flex items-center justify-center whitespace-nowrap rounded-md text-sm font-medium bg-primary text-primary-foreground h-10 px-4 py-2 hover:bg-primary/90 disabled:opacity-50"
              >
                {{ passwordLoading() ? 'Saving...' : 'Save' }}
              </button>
            </form>
          </div>
        </div>
      </div>
    </app-app-layout>
  `,
})
export class ProfileComponent {
  profileData = {
    name: '',
    email: '',
  };

  passwordData = {
    currentPassword: '',
    password: '',
    passwordConfirmation: '',
  };

  profileStatus = signal<string | null>(null);
  passwordStatus = signal<string | null>(null);
  profileLoading = signal(false);
  passwordLoading = signal(false);

  constructor(private authService: AuthService) {
    this.profileData.name = authService.user()?.name || '';
    this.profileData.email = authService.user()?.email || '';
  }

  async updateProfile() {
    this.profileLoading.set(true);
    this.profileStatus.set(null);

    try {
      await this.authService.updateProfile(this.profileData.name, this.profileData.email);
      this.profileStatus.set('Profile updated successfully!');
    } catch (error: unknown) {
      const err = error as { error?: { message?: string } };
      this.profileStatus.set(err.error?.message || 'Failed to update profile');
    } finally {
      this.profileLoading.set(false);
    }
  }

  async updatePassword() {
    this.passwordLoading.set(true);
    this.passwordStatus.set(null);

    try {
      await this.authService.updatePassword(
        this.passwordData.currentPassword,
        this.passwordData.password,
        this.passwordData.passwordConfirmation
      );
      this.passwordData = { currentPassword: '', password: '', passwordConfirmation: '' };
      this.passwordStatus.set('Password updated successfully!');
    } catch (error: unknown) {
      const err = error as { error?: { message?: string } };
      this.passwordStatus.set(err.error?.message || 'Failed to update password');
    } finally {
      this.passwordLoading.set(false);
    }
  }
}
