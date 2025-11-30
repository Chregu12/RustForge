import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { authApi, type User } from '@/lib/api'

export const useAuthStore = defineStore('auth', () => {
  const user = ref<User | null>(null)
  const loading = ref(true)

  const isAuthenticated = computed(() => !!user.value)

  async function checkAuth() {
    const token = localStorage.getItem('token')
    if (!token) {
      loading.value = false
      return
    }

    try {
      user.value = await authApi.getUser()
    } catch {
      localStorage.removeItem('token')
    } finally {
      loading.value = false
    }
  }

  async function login(email: string, password: string, remember?: boolean) {
    const response = await authApi.login({ email, password, remember })
    user.value = response.user
  }

  async function register(name: string, email: string, password: string, passwordConfirmation: string) {
    const response = await authApi.register({
      name,
      email,
      password,
      password_confirmation: passwordConfirmation,
    })
    user.value = response.user
  }

  async function logout() {
    await authApi.logout()
    user.value = null
  }

  async function refreshUser() {
    user.value = await authApi.getUser()
  }

  return {
    user,
    loading,
    isAuthenticated,
    checkAuth,
    login,
    register,
    logout,
    refreshUser,
  }
})
