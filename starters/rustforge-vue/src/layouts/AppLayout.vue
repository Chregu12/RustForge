<script setup lang="ts">
import { ref } from 'vue'
import { RouterLink, useRoute, useRouter } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import Button from '@/components/ui/Button.vue'

const route = useRoute()
const router = useRouter()
const authStore = useAuthStore()
const dropdownOpen = ref(false)

const navItems = [
  { href: '/dashboard', label: 'Dashboard' },
]

const handleLogout = async () => {
  await authStore.logout()
  router.push('/login')
}

const isActive = (href: string) => route.path === href
</script>

<template>
  <div class="min-h-screen bg-gray-100 dark:bg-gray-900">
    <!-- Navigation -->
    <nav class="bg-white dark:bg-gray-800 border-b border-gray-100 dark:border-gray-700">
      <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
        <div class="flex justify-between h-16">
          <div class="flex">
            <!-- Logo -->
            <div class="shrink-0 flex items-center">
              <RouterLink to="/dashboard" class="flex items-center gap-2">
                <svg class="w-8 h-8 text-primary" viewBox="0 0 24 24" fill="currentColor">
                  <path d="M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5" />
                </svg>
                <span class="font-bold text-gray-900 dark:text-white">RustForge</span>
              </RouterLink>
            </div>

            <!-- Navigation Links -->
            <div class="hidden space-x-8 sm:-my-px sm:ml-10 sm:flex">
              <RouterLink
                v-for="item in navItems"
                :key="item.href"
                :to="item.href"
                :class="[
                  'inline-flex items-center px-1 pt-1 border-b-2 text-sm font-medium leading-5 transition duration-150 ease-in-out',
                  isActive(item.href)
                    ? 'border-primary text-gray-900 dark:text-white'
                    : 'border-transparent text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300 hover:border-gray-300 dark:hover:border-gray-700'
                ]"
              >
                {{ item.label }}
              </RouterLink>
            </div>
          </div>

          <!-- User Dropdown -->
          <div class="hidden sm:flex sm:items-center sm:ml-6 relative">
            <Button variant="ghost" @click="dropdownOpen = !dropdownOpen" class="flex items-center gap-2">
              {{ authStore.user?.name }}
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
              </svg>
            </Button>

            <div
              v-if="dropdownOpen"
              class="absolute right-0 top-full mt-2 w-48 rounded-md shadow-lg bg-white dark:bg-gray-800 ring-1 ring-black ring-opacity-5 z-50"
              @click="dropdownOpen = false"
            >
              <div class="py-1">
                <RouterLink
                  to="/profile"
                  class="block px-4 py-2 text-sm text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700"
                >
                  Profile
                </RouterLink>
                <hr class="my-1 border-gray-200 dark:border-gray-700" />
                <button
                  @click="handleLogout"
                  class="block w-full text-left px-4 py-2 text-sm text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700"
                >
                  Log Out
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </nav>

    <!-- Page Content -->
    <main class="py-12">
      <div class="max-w-7xl mx-auto sm:px-6 lg:px-8">
        <slot />
      </div>
    </main>
  </div>
</template>
