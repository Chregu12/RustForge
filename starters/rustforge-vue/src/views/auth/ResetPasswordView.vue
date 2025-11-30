<script setup lang="ts">
import { ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { authApi } from '@/lib/api'
import GuestLayout from '@/layouts/GuestLayout.vue'
import Button from '@/components/ui/Button.vue'
import Input from '@/components/ui/Input.vue'
import Label from '@/components/ui/Label.vue'

const route = useRoute()
const router = useRouter()

const token = route.params.token as string
const email = ref((route.query.email as string) || '')
const password = ref('')
const passwordConfirmation = ref('')
const error = ref<string | null>(null)
const loading = ref(false)

const handleSubmit = async () => {
  error.value = null
  loading.value = true

  try {
    await authApi.resetPassword({
      token,
      email: email.value,
      password: password.value,
      password_confirmation: passwordConfirmation.value,
    })
    router.push({ name: 'login', query: { message: 'Password has been reset successfully!' } })
  } catch (err: unknown) {
    const e = err as { response?: { data?: { message?: string } } }
    error.value = e.response?.data?.message || 'Failed to reset password'
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <GuestLayout>
    <h2 class="text-2xl font-bold text-center text-gray-900 dark:text-white mb-6">
      Reset your password
    </h2>

    <form @submit.prevent="handleSubmit" class="space-y-4">
      <div v-if="error" class="bg-red-50 dark:bg-red-900/50 text-red-600 dark:text-red-400 p-3 rounded-md text-sm">
        {{ error }}
      </div>

      <div class="space-y-2">
        <Label for="email">Email</Label>
        <Input id="email" type="email" v-model="email" placeholder="you@example.com" required />
      </div>

      <div class="space-y-2">
        <Label for="password">New Password</Label>
        <Input id="password" type="password" v-model="password" placeholder="••••••••" required :minlength="8" autofocus />
      </div>

      <div class="space-y-2">
        <Label for="password_confirmation">Confirm Password</Label>
        <Input id="password_confirmation" type="password" v-model="passwordConfirmation" placeholder="••••••••" required />
      </div>

      <Button type="submit" class="w-full" :disabled="loading">
        {{ loading ? 'Resetting...' : 'Reset Password' }}
      </Button>
    </form>
  </GuestLayout>
</template>
