import { inject } from '@angular/core';
import { CanActivateFn, Router } from '@angular/router';
import { AuthService } from '../services/auth.service';

export const authGuard: CanActivateFn = async () => {
  const authService = inject(AuthService);
  const router = inject(Router);

  // Wait for auth check to complete
  while (authService.loading()) {
    await new Promise(resolve => setTimeout(resolve, 50));
  }

  if (!authService.isAuthenticated) {
    router.navigate(['/login']);
    return false;
  }

  return true;
};
