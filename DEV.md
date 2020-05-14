## GOTCHAS

* Longshot model is still present because, if we run into perf problems, we can switch to `tokio::spawn` immediately.
  But be careful to not do nested spawns, as it is really messy to manage everythign.

