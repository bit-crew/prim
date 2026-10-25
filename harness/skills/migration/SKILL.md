---
name: pipeline-migration
description: >
  Standard process for migrating data pipelines between technologies (Cloudera/Hive → AWS EMR,
  on-prem → cloud, batch → streaming, relational → Data Lake, cloud → cloud, etc.).
  Use whenever the user mentions migration of pipelines, ETL, data jobs,
  Spark notebooks, SQL/Scala/Python scripts that need porting to another platform,
  or when asking how to adapt legacy queries/jobs to a new environment.
---

# Pipeline Migration Skill

Universal guide for migrating data pipelines between any pair of technologies.
The process is consistent regardless of origin and destination — only the technical details change. Refer to the corresponding section in `references/` based on the detected migration type.

---

## Strict Order of Execution

### Step 1: Legacy Analysis (Priority 4: Simplification / YAGNI)

1. Read the complete legacy script/job/notebook.
2. Identify ALL data sources with their full identifiers (`schema.table`, `topic`, `bucket/path`, `database.table`, etc.).
3. Identify used columns/fields from each source (JOINs, SELECTs, filters, aggregations).
4. Document business logic (transformations, filters, rules, enrichments).
5. Identify the destination and its expected schema (columns, types, partitioning, format).
6. Identify external dependencies (catalogs, UDFs, secrets, configurations).

### Step 2: Source Verification in Destination (Priority 1: Correctness)

For EACH source identified in Step 1:

1. Locate the source in the destination system (see specific commands in `references/`).
2. If it doesn't exist with the legacy name, search for equivalents (singular/plural, prefixes, aliases).
3. For each found source, obtain:
   - Full identifier in destination.
   - Schema / field structure.
   - Data sample (3-5 records).
   - Approximate volume.
4. Document the mapping: `legacy_name → destination_name → available_fields`.
5. Identify missing or renamed fields.
6. Identify data type differences.

**Expected Outcome:** A complete mapping table before writing any code.

### Step 3: Migrated Pipeline Design

1. Define the output schema (fields, types, partitioning, file format).
2. Map each legacy JOIN/relationship to real destination sources.
3. For each missing field, decide: `NULL`, derive, or remove — document the decision.
4. Define partitioning strategy and processing time window.
5. Identify transformations requiring adaptation (UDFs, proprietary functions, SQL dialects).
6. Document differences vs. legacy and their justification.

### Step 4: Standards Alignment

Apply team/organization rules. See the standards section in `references/` based on destination technology. At a minimum, verify:

- Required audit columns.
- Naming conventions.
- Secret management (never hardcoded).
- Destination-specific read/write optimizations.
- Restrictions on UDFs or unapproved functions.
- Permissions and access roles.

### Step 5: Implementation (Priority 2: Cost & Resource Optimization)

1. Write code following the Step 3 design.
2. Handle optional or missing sources with fallback to NULL — never assume existence.
3. Use native destination optimizations (see `references/`).
4. Single artifact per complete pipeline unless volume dictates otherwise.
5. Configure resources appropriate for the estimated volume.

### Step 6: Validation (Priority 1: Correctness)

1. Execute with a minimum window (1 day or small sample) to validate logic.
2. Verify record count vs. source.
3. Verify distribution by key dimensions (date, channel, type, etc.).
4. Verify that enrichment fields are not 100% NULL.
5. Compare sample with legacy result if available.
6. Scale gradually: minimum → week → month → full window.

### Step 7: Resource Optimization (Priority 2: Cost & Resource Optimization)

1. Measure real volume per processed period.
2. Adjust resource configuration (memory, parallelism, partitions).
3. If memory/disk fails: process in batches (day-by-day, month-by-month).
4. Monitor execution plan to detect unnecessary shuffles or skew.
5. Verify that broadcasts/lookups do not exceed engine limits.

---

## Universal Migration Rules

- **NEVER** assume a field exists — verify against the real destination schema.
- **NEVER** assume source names are identical in destination — always verify.
- **NEVER** run the full job without first validating with minimum data.
- **ALWAYS** document differences between legacy and migrated versions.
- **ALWAYS** verify data types (epoch ms vs timestamp vs date vs string, etc.).
- **ALWAYS** explicitly handle joins between sources with different schemas.
- Enrichment JOINs must verify field existence before use.
- Secrets and credentials never in code — use the destination's secret manager.

---

## Pre-Execution Checklist

- [ ] All sources verified and documented with their destination name.
- [ ] All JOINs use fields that exist in the real DataFrame/table.
- [ ] Compatible data types in JOIN conditions.
- [ ] Read optimizations applied (pushdown, broadcast, etc.).
- [ ] Partition expression generates the correct format.
- [ ] Resources configured for estimated volume.
- [ ] Small window test successful before scaling.
- [ ] Audit columns present.
- [ ] No hardcoded secrets.

---

## Documentation of Differences

Upon completion of each migration, record:

```
Legacy table/source:     <legacy_name>
Destination table/source: <destination_name>
Removed fields:          <field> — reason
New fields:              <field> — origin
Renamed fields:          <legacy> → <destination>
Changed types:           <field>: <legacy_type> → <destination_type> — impact
Modified logic:          <description of change and reason>
Pending decisions:       <open question for the data team>
```
