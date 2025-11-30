<script setup lang="ts">
import { ref } from 'vue'
import { RouterLink, useRouter } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import GuestLayout from '@/layouts/GuestLayout.vue'
import Button from '@/components/ui/Button.vue'
import Input from '@/components/ui/Input.vue'
import Label from '@/components/ui/Label.vue'

const router = useRouter()
const authStore = useAuthStore()

const email = ref('')
const password = ref('')
const remember = ref(false)
const error = ref<string | null>(null)
const loading = ref(false)

const handleSubmit = async () => {
  error.value = null
  loading.value = true

  try {
    await authStore.login(email.value, password.value, remember.value)
    router.push('/dashboard')
  } catch (err: unknown) {
    const e = err as { response?: { data?: { message?: string } } }
    error.value = e.response?.data?.message || 'Invalid credentials'
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <GuestLayout>
    <h2 class="text-2xl font-bold text-center text-gray-900 dark:text-white mb-6">
      Sign in to your account
    </h2>

    <form @submit.prevent="handleSubmit" class="space-y-4">
      <div v-if="error" class="bg-red-50 dark:bg-red-900/50 text-red-600 dark:text-red-400 p-3 rounded-md text-sm">
        {{ error }}
      </div>

      <div class="space-y-2">
        <Label for="email">Email</Label>
        <Input id="email" type="email" v-model="email" placeholder="you@example.com" required autofocus />
      </div>

      <div class="space-y-2">
        <Label for="password">Password</Label>
        <Input id="password" type="password" v-model="password" placeholder="••••••••" required />
      </div>

      <div class="flex items-center justify-between">
        <label class="flex items-center">
          <input
            type="checkbox"
            v-model="remember"
            class="rounded border-gray-300 text-primary shadow-sm focus:ring-primary"
          />
          <span class="ml-2 text-sm text-gray-600 dark:text-gray-400">Remember me</span>
        </label>

        <RouterLink to="/forgot-password" class="text-sm text-primary hover:underline">
          Forgot password?
        </RouterLink>
      </div>

      <Button type="submit" class="w-full" :disabled="loading">
        {{ loading ? 'Signing in...' : 'Sign in' }}
      </Button>

      <p class="text-center text-sm text-gray-600 dark:text-gray-400">
        Don't have an account?
        <RouterLink to="/register" class="text-primary hover:underline">Sign up</RouterLink>
      </p>
    </form>
  </GuestLayout>
</template>
