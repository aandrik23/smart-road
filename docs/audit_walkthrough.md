# AUDIT_WALKTHROUGH.md

## Purpose

This document provides a complete audit procedure for validating the traffic intersection simulation.

The goal is to verify:

* Vehicle spawning behavior
* Intersection assets and rendering
* Route assignment
* Collision avoidance
* Traffic management
* Vehicle velocity logic
* Statistics generation
* Input throttling / anti-spam protections

---

# Environment Setup

## 1. Build and Run

Start the application using the project's normal execution method.

Examples:

```bash
go run .
```

or

```bash
go run main.go
```

or the project's documented launch command.

Wait until the simulation window is fully rendered.

---

# Visual Validation

## Test 1: Intersection Rendering

### Steps

1. Launch the application.
2. Observe the center of the simulation.

### Expected Result

* A cross intersection is visible.
* Roads are clearly displayed.
* Cardinal directions can be identified:

  * North
  * South
  * East
  * West

### Verify

* [ ] Cross intersection exists
* [ ] Roads are rendered correctly

---

## Test 2: Intersection Asset Validation

### Steps

Inspect project assets.

Look for:

```text
assets/
sprites/
images/
textures/
resources/
```

### Expected Result

An image, sprite, texture, or drawing representing the intersection exists.

### Verify

* [ ] Intersection asset exists

---

# Vehicle Spawn Validation

## Test 3: Arrow Up Spawn

### Steps

Press:

```text
Arrow Up
```

### Expected Result

A vehicle is generated from:

```text
South
```

moving northbound.

### Verify

* [ ] Vehicle spawned from South

---

## Test 4: Arrow Down Spawn

### Steps

Press:

```text
Arrow Down
```

### Expected Result

A vehicle is generated from:

```text
North
```

moving southbound.

### Verify

* [ ] Vehicle spawned from North

---

## Test 5: Arrow Right Spawn

### Steps

Press:

```text
Arrow Right
```

### Expected Result

A vehicle is generated from:

```text
West
```

moving eastbound.

### Verify

* [ ] Vehicle spawned from West

---

## Test 6: Arrow Left Spawn

### Steps

Press:

```text
Arrow Left
```

### Expected Result

A vehicle is generated from:

```text
East
```

moving westbound.

### Verify

* [ ] Vehicle spawned from East

---

# Random Spawn Validation

## Test 7: Random Vehicle Generation

### Steps

Press:

```text
R
```

multiple times.

### Expected Result

Vehicles are generated with:

* Random lane
* Random route
* Random direction

### Verify

* [ ] Random lane selection
* [ ] Random route selection

---

# Same Lane Stress Tests

## Test 8: Three Vehicles Per Lane

### Steps

For EACH lane:

1. Spawn vehicle #1
2. Spawn vehicle #2
3. Spawn vehicle #3

Allow them to complete the route.

Repeat for:

* North lane
* South lane
* East lane
* West lane

### Expected Result

* No collision
* Safe following distance maintained

### Verify

* [ ] North lane passed
* [ ] South lane passed
* [ ] East lane passed
* [ ] West lane passed

---

# Collision Route Stress Tests

Repeat the following tests until conflicting routes occur naturally.

---

## Test 9: Right + Left Combination

### Steps

Generate simultaneously:

* 1 vehicle using Right
* 3 vehicles using Left

Repeat until routes intersect.

### Expected Result

No collisions.

### Verify

* [ ] Passed

---

## Test 10: Up + Left Combination

### Steps

Generate simultaneously:

* 1 vehicle using Up
* 3 vehicles using Left

Repeat until routes intersect.

### Expected Result

No collisions.

### Verify

* [ ] Passed

---

## Test 11: Up + Right Combination

### Steps

Generate simultaneously:

* 1 vehicle using Up
* 3 vehicles using Right

Repeat until routes intersect.

### Expected Result

No collisions.

### Verify

* [ ] Passed

---

## Test 12: Down + Left Combination

### Steps

Generate simultaneously:

* 1 vehicle using Down
* 3 vehicles using Left

Repeat until routes intersect.

### Expected Result

No collisions.

### Verify

* [ ] Passed

---

## Test 13: Down + Right Combination

### Steps

Generate simultaneously:

* 1 vehicle using Down
* 3 vehicles using Right

Repeat until routes intersect.

### Expected Result

No collisions.

### Verify

* [ ] Passed

---

# Heavy Traffic Validation

## Test 14: Five Up + Two Right

### Steps

Generate:

* 5 vehicles using Up
* 2 vehicles using Right

at approximately the same time.

### Expected Result

No collisions.

### Verify

* [ ] Passed

---

# Collision Avoidance Validation

## Test 15: Deliberate Conflict Creation

### Steps

At least 3 separate times:

1. Generate vehicles that would naturally collide if no avoidance system existed.
2. Observe behavior.

### Expected Result

At least one vehicle:

* Slows down
* Reduces velocity
* Waits

to avoid impact.

### Verify

* [ ] Avoidance observed
* [ ] Velocity reduction observed

---

# Long Duration Simulation

## Test 16: One Minute Random Traffic

### Steps

1. Generate vehicles using R.
2. Continue generating traffic.
3. Let simulation run for 60 seconds.

### Expected Result

* No collisions
* Traffic remains fluid

### Verify

* [ ] No collision
* [ ] Stable traffic

---

## Test 17: Congestion Analysis

### Steps

Observe lane occupancy during the 60-second test.

### Expected Result

Traffic congestion remains reasonable.

Suggested threshold:

```text
< 8 vehicles waiting in same lane
```

### Verify

* [ ] Congestion acceptable

---

# Statistics Window Validation

## Test 18: Statistics Generation

### Steps

1. Generate:

   * 2 vehicles with Up
   * 2 vehicles with Right
2. Wait until all finish.
3. Press:

```text
ESC
```

### Expected Result

Statistics window appears.

### Verify

* [ ] Statistics window displayed

---

## Test 19: Vehicle Count Statistic

### Expected Result

Statistics contain:

```text
Max number of vehicles that passed the intersection: 4
```

### Verify

* [ ] Correct count

---

## Test 20: Velocity Statistics

### Expected Result

Statistics contain:

* Max velocity
* Min velocity

### Verify

* [ ] Max velocity shown
* [ ] Min velocity shown

---

## Test 21: Time Statistics

### Expected Result

Statistics contain:

* Max pass time
* Min pass time

### Verify

* [ ] Max time shown
* [ ] Min time shown

---

## Test 22: Close Calls

### Expected Result

Statistics contain:

```text
Close Calls
```

if any occurred.

### Verify

* [ ] Close calls reported

---

# Time Accuracy Validation

## Test 23: Single Vehicle Timing

### Steps

1. Spawn one vehicle.
2. Measure manually:

   * Time from spawn
   * Until vehicle leaves intersection.
3. Exit application.

### Expected Result

Statistics show:

```text
Max Time == Min Time
```

because only one vehicle existed.

Reported value should approximately match manual measurement.

### Verify

* [ ] Max equals Min
* [ ] Timing accurate

---

# Safety System Validation

## Test 24: Spawn Spam Prevention

### Steps

Rapidly spam arrow keys.

### Expected Result

Vehicle creation is limited.

Possible implementations:

* Cooldown
* Queue
* Spawn cap

### Verify

* [ ] Spam prevention exists

---

## Test 25: Route Assignment

### Steps

Generate multiple vehicles.

Observe lane choice and path.

### Expected Result

Each vehicle:

* Has assigned route
* Follows assigned route
* Does not switch routes unexpectedly

### Verify

* [ ] Routes respected

---

## Test 26: Safe Distance Configuration

### Steps

Inspect source code.

Look for:

```text
safeDistance
minimumDistance
collisionDistance
followingDistance
```

### Expected Result

Distance is:

* Greater than zero
* Reasonable relative to vehicle size

### Verify

* [ ] Safe distance configured

---

## Test 27: Safe Distance Behavior

### Steps

Create queues of vehicles.

Observe:

* Stopping
* Slowing
* Restarting

### Expected Result

Vehicles maintain separation.

### Verify

* [ ] Distance respected
* [ ] No rear-end collision

---

## Test 28: Velocity Diversity

### Steps

Inspect vehicle definitions and runtime behavior.

### Expected Result

At least:

```text
3 distinct velocities
```

exist.

Example:

```text
Slow
Medium
Fast
```

or equivalent numeric values.

### Verify

* [ ] Three or more velocities

---

# Audit Result

## PASS

All tests pass.

No collisions observed.

Statistics accurate.

Traffic remains stable.

Routes respected.

Safe distance maintained.

---

## FAIL

Any of the following occurs:

* Any vehicle Collision
* Vehicle spawn spam
* Incorrect spawn direction
* Missing statistics
* Invalid timing
* Congestion deadlock
* Missing route adherence
* Missing velocity variation
* Missing anti-spam mechanism

Document every failure with:

* Test ID
* Reproduction steps
* Screenshot (if available)
* Relevant log output
* Relevant source file

```
```
