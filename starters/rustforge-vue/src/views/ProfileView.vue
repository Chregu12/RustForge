<script setup lang="ts">
import { ref } from 'vue'
import { useAuthStore } from '@/stores/auth'
import { authApi } from '@/lib/api'
import AppLayout from '@/layouts/AppLayout.vue'
import Card from '@/components/ui/Card.vue'
import CardHeader from '@/components/ui/CardHeader.vue'
import CardTitle from '@/components/ui/CardTitle.vue'
import CardDescription from '@/components/ui/CardDescription.vue'
import CardContent from '@/components/ui/CardContent.vue'
import Button from '@/components/ui/Button.vue'
import Input from '@/components/ui/Input.vue'
import Label from '@/components/ui/Label.vue'

const authStore = useAuthStore()

const profileData = ref({
  name: authStore.user?.name || '',
  email: authStore.user?.email || '',
})

const passwordData = ref({
  current_password: '',
  password: '',
  password_confirmation: '',
})

const profileStatus = ref<string | null>(null)
const passwordStatus = ref<string | null>(null)
const profileLoading = ref(false)
const passwordLoading = ref(false)

const handleProfileUpdate = async () => {
  profileLoading.value = true
  profileStatus.value = null

  try {
    await authApi.updateProfile(profileData.value)
    await authStore.refreshUser()
    profileStatus.value = 'Profile updated successfully!'
  } catch (error: unknown) {
    const err = error as { response?: { data?: { message?: string } } }
    profileStatus.value = err.response?.data?.message || 'Failed to update profile'
  } finally {
    profileLoading.value = false
  }
}

const handlePasswordUpdate = async () => {
  passwordLoading.value = true
  passwordStatus.value = null

  try {
    await authApi.updatePassword(passwordData.value)
    passwordData.value = {
      current_password: '',
      password: '',
      password_confirmation: '',
    }
    passwordStatus.value = 'Password updated successfully!'
  } catch (error: unknown) {
    const err = error as { response?: { data?: { message?: string } } }
    passwordStatus.value = err.response?.data?.message || 'Failed to update password'
  } finally {
    passwordLoading.value = false
  }
}
</script>

<template>
  <AppLayout>
    <div class="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle>Profile Information</CardTitle>
          <CardDescription>Update your account's profile information and email address.</CardDescription>
        </CardHeader>
        <CardContent>
          <form @submit.prevent="handleProfileUpdate" class="space-y-4">
            <div class="space-y-2">
              <Label for="name">Name</Label>
              <Input id="name" v-model="profileData.name" required />
            </div>

            <div class="space-y-2">
              <Label for="email">Email</Label>
              <Input id="email" type="email" v-model="profileData.email" required />
            </div>

            <p v-if="profileStatus" :class="['text-sm', profileStatus.includes('success') ? 'text-green-600' : 'text-red-600']">
              {{ profileStatus }}
            </p>

            <Button type="submit" :disabled="profileLoading">
              {{ profileLoading ? 'Saving...' : 'Save' }}
            </Button>
          </form>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Update Password</CardTitle>
          <CardDescription>Ensure your account is using a long, random password to stay secure.</CardDescription>
        </CardHeader>
        <CardContent>
          <form @submit.prevent="handlePasswordUpdate" class="space-y-4">
            <div class="space-y-2">
              <Label for="current_password">Current Password</Label>
              <Input id="current_password" type="password" v-model="passwordData.current_password" required />
            </div>

            <div class="space-y-2">
              <Label for="password">New Password</Label>
              <Input id="password" type="password" v-model="passwordData.password" required />
            </div>

            <div class="space-y-2">
              <Label for="password_confirmation">Confirm Password</Label>
              <Input id="password_confirmation" type="password" v-model="passwordData.password_confirmation" required />
            </div>

            <p v-if="passwordStatus" :class="['text-sm', passwordStatus.includes('success') ? 'text-green-600' : 'text-red-600']">
              {{ passwordStatus }}
            </p>

            <Button type="submit" :disabled="passwordLoading">
              {{ passwordLoading ? 'Saving...' : 'Save' }}
            </Button>
          </form>
        </CardContent>
      </Card>
    </div>
  </AppLayout>
</template>
