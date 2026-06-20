//! Queue balancing for distributing workers across queues

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Queue balancing strategy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum BalanceStrategy {
    /// No balancing - fixed worker allocation
    #[serde(rename = "false")]
    Fixed,
    /// Simple round-robin balancing
    #[default]
    Simple,
    /// Auto-balance based on queue size and metrics
    Auto,
}


/// Queue information for balancing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueInfo {
    pub name: String,
    pub pending: u64,
    pub processing: u64,
    pub completed: u64,
    pub failed: u64,
    pub throughput: f64,
    pub avg_wait_time: f64,
    pub avg_runtime: f64,
    pub current_workers: u32,
}

impl QueueInfo {
    /// Create new queue info
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            pending: 0,
            processing: 0,
            completed: 0,
            failed: 0,
            throughput: 0.0,
            avg_wait_time: 0.0,
            avg_runtime: 0.0,
            current_workers: 0,
        }
    }

    /// Calculate queue priority score (higher = needs more workers)
    pub fn priority_score(&self) -> f64 {
        // Score based on:
        // 1. Number of pending jobs (weighted heavily)
        // 2. Average wait time (if jobs are waiting long)
        // 3. Throughput (inverse - lower throughput needs help)
        let pending_score = self.pending as f64 * 10.0;
        let wait_score = self.avg_wait_time * 5.0;
        let throughput_penalty = if self.throughput > 0.0 {
            100.0 / self.throughput
        } else {
            100.0
        };

        pending_score + wait_score + throughput_penalty
    }

    /// Calculate workload (jobs per worker)
    pub fn workload_per_worker(&self) -> f64 {
        if self.current_workers == 0 {
            self.pending as f64
        } else {
            self.pending as f64 / self.current_workers as f64
        }
    }
}

/// Queue balancer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueBalancer {
    pub strategy: BalanceStrategy,
    pub min_workers: u32,
    pub max_workers: u32,
    pub balance_cooldown: u32,      // Seconds between rebalances
    pub balance_max_shift: u32,     // Max workers to move in one rebalance
}

impl QueueBalancer {
    /// Create a new queue balancer
    pub fn new(strategy: BalanceStrategy) -> Self {
        Self {
            strategy,
            min_workers: 1,
            max_workers: 10,
            balance_cooldown: 30,
            balance_max_shift: 2,
        }
    }

    /// Set minimum workers per queue
    pub fn min_workers(mut self, min: u32) -> Self {
        self.min_workers = min;
        self
    }

    /// Set maximum workers per queue
    pub fn max_workers(mut self, max: u32) -> Self {
        self.max_workers = max;
        self
    }

    /// Set balance cooldown
    pub fn cooldown(mut self, seconds: u32) -> Self {
        self.balance_cooldown = seconds;
        self
    }

    /// Set max shift
    pub fn max_shift(mut self, shift: u32) -> Self {
        self.balance_max_shift = shift;
        self
    }

    /// Balance workers across queues
    pub fn balance(&self, queues: &[QueueInfo]) -> HashMap<String, u32> {
        match self.strategy {
            BalanceStrategy::Fixed => self.balance_fixed(queues),
            BalanceStrategy::Simple => self.balance_simple(queues),
            BalanceStrategy::Auto => self.balance_auto(queues),
        }
    }

    /// Fixed balancing - maintain current worker counts
    fn balance_fixed(&self, queues: &[QueueInfo]) -> HashMap<String, u32> {
        queues
            .iter()
            .map(|q| (q.name.clone(), q.current_workers.max(self.min_workers)))
            .collect()
    }

    /// Simple round-robin balancing
    fn balance_simple(&self, queues: &[QueueInfo]) -> HashMap<String, u32> {
        if queues.is_empty() {
            return HashMap::new();
        }

        // Calculate total workers available
        let total_current: u32 = queues.iter().map(|q| q.current_workers).sum();
        let workers_per_queue = (total_current / queues.len() as u32).max(self.min_workers);

        queues
            .iter()
            .map(|q| {
                let workers = workers_per_queue.min(self.max_workers).max(self.min_workers);
                (q.name.clone(), workers)
            })
            .collect()
    }

    /// Auto-balance based on queue metrics
    fn balance_auto(&self, queues: &[QueueInfo]) -> HashMap<String, u32> {
        if queues.is_empty() {
            return HashMap::new();
        }

        // Calculate total available workers
        let total_workers: u32 = queues.iter().map(|q| q.current_workers).sum();
        if total_workers == 0 {
            return self.balance_simple(queues);
        }

        // Calculate total pending jobs
        let total_pending: u64 = queues.iter().map(|q| q.pending).sum();
        if total_pending == 0 {
            // No pending jobs, distribute evenly
            return self.balance_simple(queues);
        }

        // Calculate priority scores for each queue
        let total_score: f64 = queues.iter().map(|q| q.priority_score()).sum();

        // Allocate workers based on priority scores
        let mut result = HashMap::new();
        let mut allocated = 0u32;

        for queue in queues {
            let ratio = if total_score > 0.0 {
                queue.priority_score() / total_score
            } else {
                1.0 / queues.len() as f64
            };

            let workers = ((total_workers as f64 * ratio).round() as u32)
                .max(self.min_workers)
                .min(self.max_workers);

            result.insert(queue.name.clone(), workers);
            allocated += workers;
        }

        // Adjust if we over/under-allocated due to rounding
        if allocated != total_workers {
            self.adjust_allocation(&mut result, total_workers, allocated);
        }

        result
    }

    /// Adjust worker allocation to match total
    fn adjust_allocation(
        &self,
        allocation: &mut HashMap<String, u32>,
        target: u32,
        current: u32,
    ) {
        if current == target {
            return;
        }

        let diff = if current > target {
            -(current as i32 - target as i32)
        } else {
            target as i32 - current as i32
        };

        // Simple adjustment: add/remove from first queue
        if let Some((_name, workers)) = allocation.iter_mut().next() {
            if diff > 0 {
                *workers = (*workers + diff as u32).min(self.max_workers);
            } else if *workers > self.min_workers {
                *workers = (*workers as i32 + diff).max(self.min_workers as i32) as u32;
            }
        }
    }

    /// Check if rebalancing is needed
    pub fn needs_rebalance(&self, queues: &[QueueInfo]) -> bool {
        if queues.is_empty() {
            return false;
        }

        match self.strategy {
            BalanceStrategy::Fixed => false,
            BalanceStrategy::Simple | BalanceStrategy::Auto => {
                // Check if workload is significantly unbalanced
                let workloads: Vec<f64> = queues.iter().map(|q| q.workload_per_worker()).collect();

                let max_workload = workloads.iter().cloned().fold(0.0, f64::max);
                let min_workload = workloads
                    .iter()
                    .cloned()
                    .filter(|w| *w > 0.0)
                    .fold(f64::INFINITY, f64::min);

                if min_workload.is_infinite() {
                    return false;
                }

                // If max workload is > 2x min workload, rebalance
                max_workload > min_workload * 2.0
            }
        }
    }
}

impl Default for QueueBalancer {
    fn default() -> Self {
        Self::new(BalanceStrategy::Simple)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_queue_info_priority_score() {
        let mut queue = QueueInfo::new("test");
        queue.pending = 100;
        queue.avg_wait_time = 10.0;
        queue.throughput = 5.0;

        let score = queue.priority_score();
        assert!(score > 0.0);
    }

    #[test]
    fn test_queue_info_workload_per_worker() {
        let mut queue = QueueInfo::new("test");
        queue.pending = 100;
        queue.current_workers = 5;

        assert_eq!(queue.workload_per_worker(), 20.0);
    }

    #[test]
    fn test_balance_fixed() {
        let balancer = QueueBalancer::new(BalanceStrategy::Fixed);

        let mut q1 = QueueInfo::new("q1");
        q1.current_workers = 3;

        let mut q2 = QueueInfo::new("q2");
        q2.current_workers = 5;

        let result = balancer.balance(&[q1, q2]);
        assert_eq!(result.get("q1"), Some(&3));
        assert_eq!(result.get("q2"), Some(&5));
    }

    #[test]
    fn test_balance_simple() {
        let balancer = QueueBalancer::new(BalanceStrategy::Simple).min_workers(2);

        let mut q1 = QueueInfo::new("q1");
        q1.current_workers = 8;

        let mut q2 = QueueInfo::new("q2");
        q2.current_workers = 4;

        let result = balancer.balance(&[q1, q2]);

        // Should distribute evenly: 12 workers / 2 queues = 6 each
        assert_eq!(result.get("q1"), Some(&6));
        assert_eq!(result.get("q2"), Some(&6));
    }

    #[test]
    fn test_balance_auto() {
        let balancer = QueueBalancer::new(BalanceStrategy::Auto)
            .min_workers(1)
            .max_workers(10);

        let mut q1 = QueueInfo::new("q1");
        q1.pending = 100;
        q1.current_workers = 5;

        let mut q2 = QueueInfo::new("q2");
        q2.pending = 20;
        q2.current_workers = 5;

        let result = balancer.balance(&[q1, q2]);

        // q1 should get more workers due to higher pending count
        assert!(result.get("q1").unwrap() > result.get("q2").unwrap());
    }

    #[test]
    fn test_needs_rebalance() {
        let balancer = QueueBalancer::new(BalanceStrategy::Auto);

        let mut q1 = QueueInfo::new("q1");
        q1.pending = 100;
        q1.current_workers = 2; // 50 jobs/worker

        let mut q2 = QueueInfo::new("q2");
        q2.pending = 10;
        q2.current_workers = 2; // 5 jobs/worker

        // Max workload (50) > 2x min workload (5), should rebalance
        assert!(balancer.needs_rebalance(&[q1, q2]));
    }

    #[test]
    fn test_balance_strategy_serialization() {
        let strategy = BalanceStrategy::Auto;
        let json = serde_json::to_string(&strategy).unwrap();
        assert_eq!(json, "\"auto\"");

        let strategy = BalanceStrategy::Fixed;
        let json = serde_json::to_string(&strategy).unwrap();
        assert_eq!(json, "\"false\"");
    }

    #[test]
    fn test_balancer_builder() {
        let balancer = QueueBalancer::new(BalanceStrategy::Auto)
            .min_workers(2)
            .max_workers(20)
            .cooldown(60)
            .max_shift(5);

        assert_eq!(balancer.min_workers, 2);
        assert_eq!(balancer.max_workers, 20);
        assert_eq!(balancer.balance_cooldown, 60);
        assert_eq!(balancer.balance_max_shift, 5);
    }
}
