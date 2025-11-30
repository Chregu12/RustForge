import { Component, signal } from '@angular/core';
import { RouterLink, RouterLinkActive } from '@angular/router';
import { AuthService } from '../../services/auth.service';
import { NgClass } from '@angular/common';

@Component({
  selector: 'app-app-layout',
  standalone: true,
  imports: [RouterLink, RouterLinkActive, NgClass],
  template: `
    <div class="min-h-screen bg-gray-100 dark:bg-gray-900">
      <!-- Navigation -->
      <nav class="bg-white dark:bg-gray-800 border-b border-gray-100 dark:border-gray-700">
        <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div class="flex justify-between h-16">
            <div class="flex">
              <!-- Logo -->
              <div class="shrink-0 flex items-center">
                <a routerLink="/dashboard" class="flex items-center gap-2">
                  <svg class="w-8 h-8 text-primary" viewBox="0 0 24 24" fill="currentColor">
                    <path d="M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5" />
                  </svg>
                  <span class="font-bold text-gray-900 dark:text-white">RustForge</span>
                </a>
              </div>

              <!-- Navigation Links -->
              <div class="hidden space-x-8 sm:-my-px sm:ml-10 sm:flex">
                <a
                  routerLink="/dashboard"
                  routerLinkActive="border-primary text-gray-900 dark:text-white"
                  [routerLinkActiveOptions]="{exact: true}"
                  class="inline-flex items-center px-1 pt-1 border-b-2 text-sm font-medium leading-5 transition duration-150 ease-in-out border-transparent text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300 hover:border-gray-300 dark:hover:border-gray-700"
                >
                  Dashboard
                </a>
              </div>
            </div>

            <!-- User Dropdown -->
            <div class="hidden sm:flex sm:items-center sm:ml-6 relative">
              <button
                (click)="dropdownOpen.set(!dropdownOpen())"
                class="flex items-center gap-2 text-sm font-medium text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300 transition"
              >
                {{ authService.user()?.name }}
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
                </svg>
              </button>

              @if (dropdownOpen()) {
                <div
                  class="absolute right-0 top-full mt-2 w-48 rounded-md shadow-lg bg-white dark:bg-gray-800 ring-1 ring-black ring-opacity-5 z-50"
                >
                  <div class="py-1">
                    <a
                      routerLink="/profile"
                      (click)="dropdownOpen.set(false)"
                      class="block px-4 py-2 text-sm text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700"
                    >
                      Profile
                    </a>
                    <hr class="my-1 border-gray-200 dark:border-gray-700" />
                    <button
                      (click)="logout()"
                      class="block w-full text-left px-4 py-2 text-sm text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700"
                    >
                      Log Out
                    </button>
                  </div>
                </div>
              }
            </div>
          </div>
        </div>
      </nav>

      <!-- Page Content -->
      <main class="py-12">
        <div class="max-w-7xl mx-auto sm:px-6 lg:px-8">
          <ng-content></ng-content>
        </div>
      </main>
    </div>
  `,
})
export class AppLayoutComponent {
  dropdownOpen = signal(false);

  constructor(public authService: AuthService) {}

  async logout() {
    this.dropdownOpen.set(false);
    await this.authService.logout();
  }
}
