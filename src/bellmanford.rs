use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq)]
pub struct Edge {
  pub from: String,
  pub to: String,
  pub weight: f64,
}

#[derive(Debug)]
pub struct BellmanFord {
  pub edges: Vec<Edge>,
  pub currency_index_map: HashMap<String, usize>
}

impl BellmanFord {
  pub fn new(exchange_rates: &Vec<(String, String, f64)>) -> Self {
    let mut edges = Vec::new();
    let mut currency_index_map = HashMap::new();
    let mut index = 0;

    for (from, to, rate) in exchange_rates {
      edges.push(Edge {
        from: from.clone(),
        to: to.clone(),
        weight: -f64::log10(*rate),
      });

      if !currency_index_map.contains_key(from) {
        currency_index_map.insert(from.clone(), index);
        index += 1;
      }
      if !currency_index_map.contains_key(to) {
        currency_index_map.insert(to.clone(), index);
        index += 1;
      }
    }

    Self { edges, currency_index_map }
  }

  /// Find Negative Cycle
  /// Finds a single negative cycle
  pub fn find_negative_cycle(&self) -> Option<Vec<Edge>> {
    let number_of_currencies = self.currency_index_map.len();
    let mut distance = vec![f64::INFINITY; number_of_currencies];
    let mut predecessor: Vec<Option<Edge>> = vec![None; number_of_currencies];

    distance[0] = 0.0; // Assuming 0 as the source vertex

    for _ in 0..number_of_currencies {
      let mut update = false; // Flag to check if any update happens in this iteration

      for edge in &self.edges {
          let u = self.get_currency_index(&edge.from);
          let v = self.get_currency_index(&edge.to);
          if distance[u] + edge.weight < distance[v] {
            distance[v] = distance[u] + edge.weight;
            predecessor[v] = Some(edge.clone());
            update = true; // Update flag when a change occurs
          }
      }

      // Early termination if no update in this iteration
      if !update {
        break;
      }
    }

    // Check for negative cycle
    for edge in &self.edges {
      let u = self.get_currency_index(&edge.from);
      let v = self.get_currency_index(&edge.to);
      if distance[u] + edge.weight < distance[v] {
        return Some(self.construct_cycle(v, &predecessor));
      }
    }

    None
  }

  /// Find All Negative Cycles
  /// Find all negative cycles possible
  pub fn find_all_negative_cycles(&self) -> Vec<Vec<Edge>> {
    let number_of_currencies = self.currency_index_map.len();
    let mut distance = vec![f64::INFINITY; number_of_currencies];
    let mut predecessor: Vec<Option<Edge>> = vec![None; number_of_currencies];
    let mut visited_edges = HashSet::new();

    distance[0] = 0.0; // Assuming 0 as the source vertex

    // Relax edges
    for _ in 0..number_of_currencies {
        for edge in &self.edges {
            let u = self.get_currency_index(&edge.from);
            let v = self.get_currency_index(&edge.to);
            if distance[u] + edge.weight < distance[v] {
                distance[v] = distance[u] + edge.weight;
                predecessor[v] = Some(edge.clone());
            }
        }
    }

    // Check for negative cycles
    let mut cycles = Vec::new();
    for edge in &self.edges {
        let u = self.get_currency_index(&edge.from);
        let v = self.get_currency_index(&edge.to);

        // Skip the edge if it was part of a previously found cycle
        if visited_edges.contains(&(u, v)) {
            continue;
        }

        if distance[u] + edge.weight < distance[v] {
            let cycle = self.construct_cycle(v, &predecessor);
            if !cycle.is_empty() {
                // Mark all edges in the cycle as visited
                for cycle_edge in &cycle {
                    let from_index = self.get_currency_index(&cycle_edge.from);
                    let to_index = self.get_currency_index(&cycle_edge.to);
                    visited_edges.insert((from_index, to_index));
                }

                if !cycles.contains(&cycle) {
                  cycles.push(cycle);
                }
            }
        }
    }

    cycles
  }

  /// Get Currency Index
  /// Retrieves the index for a given currency str
  fn get_currency_index(&self, currency: &str) -> usize {
    let i = self.currency_index_map.get(currency).copied().expect("Missing currency index");
    i
  }

  /// Construct Cycle
  /// Provides ordering and information for the cycle in question
  fn construct_cycle(&self, start: usize, predecessor: &Vec<Option<Edge>>) -> Vec<Edge> {
    let mut cycle = Vec::new();
    let mut visited = std::collections::HashSet::new();
    let mut current = start;

    // Find the actual start of the cycle
    let mut cycle_start = None;
    while let Some(edge) = &predecessor[current] {
      if visited.contains(&current) {
        cycle_start = Some(current);
        break;
      }
      visited.insert(current);
      current = self.get_currency_index(&edge.from);
    }

    // Reconstruct the cycle from the start
    if let Some(cycle_start_vertex) = cycle_start {
      cycle.clear();
      visited.clear();
      current = cycle_start_vertex;

      loop {
        if let Some(edge) = &predecessor[current] {
          if visited.contains(&current) {
            break;
          }
          visited.insert(current);
          cycle.push(edge.clone());
          current = self.get_currency_index(&edge.from);
        } else {
          break;
        }
      }
    }

    // Filter out cycles that are just a two-edge reciprocation
    if cycle.len() <= 2 {
      cycle.clear();
    }

    cycle.reverse();
    cycle
  }

}

#[cfg(test)]
mod tests {
  use super::*;

  fn get_test_rates_fx() -> Vec<(String, String, f64)> {
    vec![
      ("USD".to_string(), "EUR".to_string(), 0.9),
      ("EUR".to_string(), "USD".to_string(), 1.21),
      ("USD".to_string(), "GBP".to_string(), 0.75),
      ("GBP".to_string(), "USD".to_string(), 1.33),
      ("GBP".to_string(), "EUR".to_string(), 1.197),
    ]
  }

  #[tokio::test]
  async fn it_detects_arbitrage_negative_cycle() {
    let test_exchange_rates: Vec<(String, String, f64)> = get_test_rates_fx();
    let bf: BellmanFord = BellmanFord::new(&test_exchange_rates);
    match bf.find_negative_cycle() {
      Some(cycle) => {
        let mut stake = 1000.0;
        for edge in cycle {
          println!("{} to {}: {}", edge.from, edge.to, stake);
          stake *= f64::exp(-edge.weight);
        }
        println!("Final stake: {}", stake);
        assert!(stake > 1000.0);
      },
      None => println!("No arbitrage opportunity"),
    }
  }

  #[tokio::test]
  async fn it_detects_all_arbitrage_negative_cycles() {
    let test_exchange_rates = get_test_rates_fx();
    let bf: BellmanFord = BellmanFord::new(&test_exchange_rates);
    let cycles = bf.find_all_negative_cycles();
    assert!(cycles.len() > 0);
    assert!(cycles[0].len() > 0);
  }
}
