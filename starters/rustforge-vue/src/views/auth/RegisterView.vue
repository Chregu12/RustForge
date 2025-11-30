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

const name = ref('')
const email = ref('')
const password = ref('')
const passwordConfirmation = ref('')
const error = ref<string | null>(null)
const loading = ref(false)

const handleSubmit = async () => {
  error.value = null
  loading.value = true

  try {
    await authStore.register(name.value, email.value, password.value, passwordConfirmation.value)
    router.push('/dashboard')
  } catch (err: unknown) {
    const e = err as { response?: { data?: { message?: string; errors?: Record<string, string[]> } } }
    if (e.response?.data?.errors) {
      const firstError = Object.values(e.response.data.errors)[0]
      error.value = Array.isArray(firstError) ? firstError[0] : String(firstError)
    } else {
      error.value = e.response?.data?.message || 'Registration failed'
    }
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <GuestLayout>
    <h2 class="text-2xl font-bold text-center text-gray-900 dark:text-white mb-6">
      Create your account
    </h2>

    <form @submit.prevent="handleSubmit" class="space-y-4">
      <div v-if="error" class="bg-red-50 dark:bg-red-900/50 text-red-600 dark:text-red-400 p-3 rounded-md text-sm">
        {{ error }}
      </div>

      <div class="space-y-2">
        <Label for="name">Name</Label>
        <Input id="name" v-model="name" placeholder="John Doe" required autofocus />
      </div>

      <div class="space-y-2">
        <Label for="email">Email</Label>
        <Input id="email" type="email" v-model="email" placeholder="you@example.com" required />
      </div>

      <div class="space-y-2">
        <Label for="password">Password</Label>
        <Input id="password" type="password" v-model="password" placeholder="••••••••" required :minlength="8" />
      </div>

      <div class="space-y-2">
        <Label for="password_confirmation">Confirm Password</Label>
        <Input id="password_confirmation" type="password" v-model="passwordConfirmation" placeholder="••••••••" required />
      </div>

      <Button type="submit" class="w-full" :disabled="loading">
        {{ loading ? 'Creating account...' : 'Create account' }}
      </Button>

      <p class="text-center text-sm text-gray-600 dark:text-gray-400">
        Already have an account?
        <RouterLink to="/login" class="text-primary hover:underline">Sign in</RouterLink>
      </p>
    </form>
  </GuestLayout>
</template>
