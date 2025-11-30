import { Component } from '@angular/core';
import { AppLayoutComponent } from '../../layouts/app-layout/app-layout.component';
import { AuthService } from '../../services/auth.service';

@Component({
  selector: 'app-dashboard',
  standalone: true,
  imports: [AppLayoutComponent],
  template: `
    <app-app-layout>
      <div class="space-y-6">
        <div class="bg-white dark:bg-gray-800 overflow-hidden shadow-sm sm:rounded-lg">
          <div class="p-6 text-gray-900 dark:text-gray-100">
            Welcome back, <span class="font-semibold">{{ authService.user()?.name }}</span>!
          </div>
        </div>

        <div class="grid gap-6 md:grid-cols-2 lg:grid-cols-3">
          <!-- Quick Stats Card -->
          <div class="rounded-lg border bg-card text-card-foreground shadow-sm">
            <div class="flex flex-col space-y-1.5 p-6">
              <h3 class="text-2xl font-semibold leading-none tracking-tight">Quick Stats</h3>
              <p class="text-sm text-muted-foreground">Your activity overview</p>
            </div>
            <div class="p-6 pt-0">
              <div class="text-2xl font-bold">0</div>
              <p class="text-xs text-muted-foreground">Active projects</p>
            </div>
          </div>

          <!-- Recent Activity Card -->
          <div class="rounded-lg border bg-card text-card-foreground shadow-sm">
            <div class="flex flex-col space-y-1.5 p-6">
              <h3 class="text-2xl font-semibold leading-none tracking-tight">Recent Activity</h3>
              <p class="text-sm text-muted-foreground">What you've been up to</p>
            </div>
            <div class="p-6 pt-0">
              <p class="text-sm text-muted-foreground">No recent activity</p>
            </div>
          </div>

          <!-- Getting Started Card -->
          <div class="rounded-lg border bg-card text-card-foreground shadow-sm">
            <div class="flex flex-col space-y-1.5 p-6">
              <h3 class="text-2xl font-semibold leading-none tracking-tight">Getting Started</h3>
              <p class="text-sm text-muted-foreground">Build something amazing</p>
            </div>
            <div class="p-6 pt-0">
              <ul class="text-sm space-y-2 text-muted-foreground">
                <li>• Create your first model</li>
                <li>• Set up authentication</li>
                <li>• Deploy to production</li>
              </ul>
            </div>
          </div>
        </div>
      </div>
    </app-app-layout>
  `,
})
export class DashboardComponent {
  constructor(public authService: AuthService) {}
}
