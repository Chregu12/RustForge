<script setup lang="ts">
import { ref } from 'vue'
import { RouterLink } from 'vue-router'
import { authApi } from '@/lib/api'
import GuestLayout from '@/layouts/GuestLayout.vue'
import Button from '@/components/ui/Button.vue'
import Input from '@/components/ui/Input.vue'
import Label from '@/components/ui/Label.vue'

const email = ref('')
const status = ref<string | null>(null)
const error = ref<string | null>(null)
const loading = ref(false)

const handleSubmit = async () => {
  error.value = null
  status.value = null
  loading.value = true

  try {
    const response = await authApi.forgotPassword(email.value)
    status.value = response.message
  } catch (err: unknown) {
    const e = err as { response?: { data?: { message?: string } } }
    error.value = e.response?.data?.message || 'Failed to send reset link'
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <GuestLayout>
    <h2 class="text-2xl font-bold text-center text-gray-900 dark:text-white mb-4">
      Forgot your password?
    </h2>
    <p class="text-center text-sm text-gray-600 dark:text-gray-400 mb-6">
      No problem. Just let us know your email address and we will email you a password reset link.
    </p>

    <form @submit.prevent="handleSubmit" class="space-y-4">
      <div v-if="error" class="bg-red-50 dark:bg-red-900/50 text-red-600 dark:text-red-400 p-3 rounded-md text-sm">
        {{ error }}
      </div>

      <div v-if="status" class="bg-green-50 dark:bg-green-900/50 text-green-600 dark:text-green-400 p-3 rounded-md text-sm">
        {{ status }}
      </div>

      <div class="space-y-2">
        <Label for="email">Email</Label>
        <Input id="email" type="email" v-model="email" placeholder="you@example.com" required autofocus />
      </div>

      <Button type="submit" class="w-full" :disabled="loading">
        {{ loading ? 'Sending...' : 'Email Password Reset Link' }}
      </Button>

      <p class="text-center text-sm text-gray-600 dark:text-gray-400">
        Remember your password?
        <RouterLink to="/login" class="text-primary hover:underline">Sign in</RouterLink>
      </p>
    </form>
  </GuestLayout>
</template>
