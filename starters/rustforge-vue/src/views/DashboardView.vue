<script setup lang="ts">
import { useAuthStore } from '@/stores/auth'
import AppLayout from '@/layouts/AppLayout.vue'
import Card from '@/components/ui/Card.vue'
import CardHeader from '@/components/ui/CardHeader.vue'
import CardTitle from '@/components/ui/CardTitle.vue'
import CardDescription from '@/components/ui/CardDescription.vue'
import CardContent from '@/components/ui/CardContent.vue'

const authStore = useAuthStore()
</script>

<template>
  <AppLayout>
    <div class="space-y-6">
      <div class="bg-white dark:bg-gray-800 overflow-hidden shadow-sm sm:rounded-lg">
        <div class="p-6 text-gray-900 dark:text-gray-100">
          Welcome back, <span class="font-semibold">{{ authStore.user?.name }}</span>!
        </div>
      </div>

      <div class="grid gap-6 md:grid-cols-2 lg:grid-cols-3">
        <Card>
          <CardHeader>
            <CardTitle>Quick Stats</CardTitle>
            <CardDescription>Your activity overview</CardDescription>
          </CardHeader>
          <CardContent>
            <div class="text-2xl font-bold">0</div>
            <p class="text-xs text-muted-foreground">Active projects</p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Recent Activity</CardTitle>
            <CardDescription>What you've been up to</CardDescription>
          </CardHeader>
          <CardContent>
            <p class="text-sm text-muted-foreground">No recent activity</p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Getting Started</CardTitle>
            <CardDescription>Build something amazing</CardDescription>
          </CardHeader>
          <CardContent>
            <ul class="text-sm space-y-2 text-muted-foreground">
              <li>• Create your first model</li>
              <li>• Set up authentication</li>
              <li>• Deploy to production</li>
            </ul>
          </CardContent>
        </Card>
      </div>
    </div>
  </AppLayout>
</template>
