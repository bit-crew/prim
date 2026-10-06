# System

## INPUT
### SYMPTOM: Dispersion of Efforts (Lack of impact)
- WHY
  - 1. Too many initiatives at once
    - 2. Lack of Criteria (Everything is urgent)
      - 3. **Excess of Indicators (Noise)** <small><small>
          **DATA:** 95% of employees do not understand the strategy *(HB Review)* </small></small>
        - STRATEGY 
          - A. Radical Focus (Pareto)
          - B. Single indicator per team (Amazon) <small><small>
            `Visualization: Bar/Line (0-1)` 
            `Traffic Light: Green/Amber/Red`
            **[AGENT - Guardian]** `Block creation of new KPIs if active KPIs already exist` </small></small>
          - C. OKR Protocol *(Intel | Andy Grove, 1971)* <small><small>
            `Maximum 3 Key Results | Weighted sum = 1`
            `Mandatory weekly review`
            `Target: Score between 0.7 - 0.9`
            **[AGENT - Monitor]** `Alert if Score < 0.7: "Objective at risk"`
            `Alert if Score > 0.9: "Increase challenge level"`
            `Calculate automatic weekly progress`
            `Generate review report every Monday` </small></small>

### SYMPTOM: Collapse by Saturation
- WHY
  - 1. The system operates slowly or locks up
    - 2. Obsession with Occupation (100%)
      - 3. Delusional optimism in deadlines
        - 4. **Assuming the best possible scenario** *(Kahneman & Tversky, 1979)* <small><small>
          **DATA:** 99% of estimates fail in 55% of projects *(Kahneman & Tversky, 1979)* </small></small> 
      - 3. Queue physics is ignored   
        - 4. **Queues - Gridlock at 99%** *(Kingman's Law)* <small><small>
          **DATA:** At 90% occupancy wait multiplies x9 and At 99% the system collapses </small></small>
        - STRATEGY
          - A. Planned Slack <small><small>
            `Rule: Max 80% Planned Occupancy` 
            **[AGENT - Monitor]** `Calculate aggregate future load per person/team`
            `If projected load > 80%, block assignment of new tasks`
            `Suggest moving deadline or reassigning resources` </small></small>            



### SYMPTOM: Erratic Decisions (Whiplash)
- WHY
  - 1. Reaction to immediate pain (attacking Symptoms)
    - 2. Judgment degradation (Bio-Rhythm)
      - 3. **Biological Limits (Amygdala Hijack)**
      - STRATEGY
        - Segregated Schedule <small><small>
          `Critical Decisions: 08:00-11:00 AM`
          `Sync Meetings: Only PM` 
          **[AGENT - Guardian]** `Block meeting invites to technical roles before 14:00` </small></small> 
  - 1. **Lack of Mental Models**
    - STRATEGY
      - Algorithmic Governance *(Principles | Ray Dalio, 2018)* <small><small>
        `Loop: Diagnosis -> Design -> Execution`
        `Rule: Adjust the rule, not the specific decision`
        `Mathematical or statistical basis`
        **[AGENT - Assistant]** `Repository of documented "Principles"`
        `Faced with a repeated problem, suggest the pre-loaded rule` </small></small>  

### SYMPTOM: Intake of Toxic Assets 
- WHY 
  - 1. Low quality entry is permitted 
    - 2. Short-term Vision (Misunderstood savings) 
      - 3. **Optimize CAPEX ignoring OPEX (TCO)** - STRATEGY 
          - A. Quality Standards (Minimalism) <small><small>
            `Principle: Buy nice or buy twice` 
            **[AGENT - Guardian]** `Enforce strict "Definition of Done" before purchase or deploy`
            `Block entry of code/assets without fulfilling checklist` </small></small>

### SYMPTOM: Rudderless (no direction / wrong market)
- WHY
  - 1. Effort is executed without a defined destination
    - 2. Strategy assumed, never made explicit
      - 3. Vision lives in the founder's head, not in the SoT
        - 4. **No single, written, queryable North Star** *(EOS "Vision" component | Wickman)* <small><small>
          **DATA:** "No market need" is the #1 root cause of failure — 42% of post-mortems *(CB Insights)* </small></small>
          - STRATEGY
            - A. Explicit North Star (Backend-First) <small><small>
              `One written vision doc in the SoT (Git), versioned`
              `Every initiative must link to a vision objective (traceable)`
              **[AGENT - Guardian]** `Block creation of an initiative with no linked vision objective`
              `On new project: require a one-line "why this, why now" tied to the North Star`
              **[AGENT - Assistant]** `Answer "does this align with the vision?" from the RAG` </small></small>

### SYMPTOM: Building What Nobody Wants (no PMF)
- WHY
  - 1. Internal execution is optimized before demand is proven
    - 2. Founder falls in love with the solution
      - 3. No customer/demand validation gate before building
        - 4. **Solution in search of a problem** *(Blank | Customer Development)* <small><small>
          **DATA:** Poor product-market fit 43% + unsustainable unit economics 19% *(CB Insights)* </small></small>
          - STRATEGY
            - A. Demand-First Gate (Minimalism / anti-waste) <small><small>
              `Rule: no build without a validated demand signal (paying intent or usage)`
              `Track unit economics per product (does one unit pay for itself?)`
              **[AGENT - Guardian]** `Block major build effort if no demand-validation record exists`
              **[AGENT - Monitor]** `Alert if a product's unit economics turn negative`
              `Alert if effort is spent on a product with no demand signal in N weeks` </small></small>

### SYMPTOM: Redundant Work (teams build the same thing)
- WHY
  - 1. Several teams develop overlapping/duplicate solutions
    - 2. They do not know what already exists (ignorance across teams)
      - 3. Knowledge is fragmented in silos (each team its own repo/data)
        - 4. No single source of truth consulted *before* building
          - 5. **Absence of prior discovery** — nobody checks the unified knowledge before creating <small><small>
            **DATA:** 61% of workers regularly recreate work that already exists *(Guru, 2024)*; 14% of time lost recreating unfound information *(Deloitte)* </small></small>
            - STRATEGY
              - A. Discovery-Before-Build (Prim thesis: unified data + agent) <small><small>
                `SoT: all projects/repos/data unified in Cloudflare (R2 + RAG index)`
                `Rule: creating a project/product requires a discovery step first`
                **[AGENT - Guardian]** `On new project/product: semantic-similarity search over the unified RAG`
                `If similarity > threshold, surface the existing effort and require justification to proceed`
                **[AGENT - Monitor]** `Continuously detect convergent efforts across teams (push alert, not dashboard)`
                `Alert both owners when two active efforts overlap semantically`
                `Suggest merge/reuse instead of parallel rebuild` </small></small>

## PROCESS

### SYMPTOM: Communicational Friction
- WHY
  - 1. Need for constant synchronization
    - 2. Oral Culture
      - 3. **Cognitive Friction (speaking < writing)**
        - STRATEGY <small><small>
          **NOTE:** `Self-explained Content` </small></small>
          - Narrative Memo (6 pages) *(Amazon)* <small><small>
            `Structure: Context + Objective + Data`
            `Format: Narrative prose, not bullet points`
            `Rule: 24h mandatory prior reading`
            `Version control accessible to all stakeholders`
            **[AGENT - Assistant]** `Intercept invitations > 3 people`
            `Demand attaching Memo when scheduling meeting`
            `Offer standardized template`
            `Review syntax and clarity of the Memo (document linter)`
            `Meeting protocol:`
            `  - 5 initial minutes of silent re-reading`
            `  - Meeting limited to Q&A, not presentation`
            `  - Update memo post-meeting with decisions` </small></small>    

### SYMPTOM: Corporate Theater
- WHY
  - 1. Obsession with BI (Corporate Theater) 
    - 2. **"Corporate Porn"**: Graphics to look like one has control. Real Value: Zero.
      - 3. **Green Traffic Light Fallacy**: Manipulating KPIs to soothe anxiety.
        - 4. **Garbage In, HD Out**: Dirty data presented in 4K.
          - 5. **Analysis Paralysis**: "Let's wait to have more data" to avoid deciding.
            - STRATEGY
              - A. Management by Exception (No Dashboards) <small><small>
                `Operational Monitoring (Raw Data): Is the engine on fire?`
                `Replace UI (Pull) by Alerts (Push)` 
                **[AGENT - Monitor]** `Eliminate automatic visual report`
                `Notify only if KPI < 0.7 or anomaly detected`
                `Send raw data, not graphics` </small></small>

### SYMPTOM: Low Productivity (Throughput Loss)
- WHY
  - 1. Loss of focus (Deep Work)
    - 2. Interruptions without limits (Context Switching)
      - 3. Anyone can schedule a meeting or request something
        - 4. **Cost of Context Switching is disregarded** <small><small>
          **DATA:** 23 min 15 sec to recover focus *(Gloria Mark, Univ. California)*
          Multitasking reduces functional productivity by 40% *(APA)* </small></small> 
        - STRATEGY
          - Focus Protection *(Deep Work | Cal Newport)* <small><small>
            `Wednesday: No Meeting Wednesday`
            `Request Firewall (Only PO receives external requests)` 
            `Communication hierarchy: Async Msg -> Chat -> Email -> Call`
            `Meeting limits per role:`
            `  - PO: Unlimited`
            `  - Tech Lead: Max 3h/day`
            `  - Technical role: Max 6h/week`
            **[AGENT - Guardian]** `Automatically reject meetings on Wednesday`
            `Reject meetings for technical roles before 14:00 or after 15:00`
            `Only PO can receive requests from other teams`
            `Block direct invites to technical roles without Tech Lead approval`
            `Calculate meeting hours per role (rolling 7 days)`
            `Block if weekly limit is exceeded` </small></small> 

### SYMPTOM: Low Execution Velocity
- WHY
  - 1. Latency in decisions
    - 2. **Personnel with little resolve / Excess of personnel** <small><small>
      **DATA:** Latency costs $3,750 per employee annually *(McKinsey)* </small></small> 
      - STRATEGY
        - Talent Density <small><small>
          `Teams < 10 people` *(Two Pizza Rule | Jeff Bezos)*
          `Pay Top of the market` *(Keeper Test | Netflix, 2009)* </small></small> 
  - 1. Dependency on key people
    - 2. The system stops when someone is absent 
      - 3. **Identity tied to the Person, not the Role**
        - STRATEGY
          - Role Management (RBAC) <small><small>
            `Permissions and tasks are assigned per project and per role`
            `Documented Rotation/Vacation protocol`
            **[AGENT - Assistant]** `Main business context expert (RAG over documentation)`
            `Assistance in alignment with operational principles`
            `Support in specific doubts with historical context`
            `Automatic "Onboarding/Offboarding" script:`
            `  - Automatic role reassignment`
            `  - Transfer of active projects`
            `  - Reassignment of orphan tasks` </small></small>         

### SYMPTOM: Organizational Amnesia (does not learn)
- WHY
  - 1. The same problems are solved from scratch again and again
    - 2. Lessons are not captured where they are searched
      - 3. Knowledge leaves with the person / lives in chats, not the SoT
        - 4. **No systemic learning loop** — capture is ad-hoc, retrieval is manual <small><small>
          **DATA:** Only 27% have a reliable maintained knowledge base *(Guru, 2024)*; employees spend ~19% of the week searching for information *(McKinsey)* </small></small>
          - STRATEGY
            - A. Learning as Flow (RAG-native, not an event) <small><small>
              `Every post-mortem, decision and principle indexed into the unified RAG`
              `Channel history is a first-class RAG source (per GOVERNANCE.md)`
              `Rule: a resolved incident must emit a reusable "principle", not just a fix`
              **[AGENT - Assistant]** `On a new problem, retrieve prior solutions/principles before work starts`
              **[AGENT - Guardian]** `Flag when a "new" task matches a previously solved one` </small></small>

### SYMPTOM: Regulatory/Legal Risk Ignored
- WHY
  - 1. Compliance is treated as an afterthought
    - 2. Tax/legal obligations tracked informally (or not at all)
      - 3. No owner and no calendar for statutory deadlines
        - 4. **Silent accrual of legal/fiscal liability** (fines, dissolution risk) <small><small>
          **NOTE:** `SAS obligations documented in docs/guia-tributaria-sas.md` </small></small>
          - STRATEGY
            - A. Compliance-by-Calendar (Push, no dashboard) <small><small>
              `Statutory/tax deadlines as first-class scheduled events (SoT)`
              `Map each obligation to a responsible role (RBAC)`
              **[AGENT - Monitor]** `Alert ahead of every statutory/tax deadline (lead time configurable)`
              `Escalate if an obligation has no owner`
              **[AGENT - Assistant]** `Answer "what do we owe, to whom, by when" from the fiscal docs + calendar` </small></small>

## OUTPUT

### SYMPTOM: System Degradation (Entropy) 
- WHY 
  - 1. The system gets dirty with use 
    - 2. Lack of Continuous Maintenance 
      - 3. **Compound Interest of Debt** <small><small>
        **DATA:** Consumes 41% of dev time *(Stripe)* </small></small> 
        - STRATEGY 
          - A. Technical Kaizen (Continuous Flow) <small><small>
            `Maintenance as part of the flow, not a special event`
            `If it is a recurring task, it is automated`
            `Mandatory 20% budget for maintenance`
            **[AGENT - Guardian]** `Calculate Maintenance Ratio (rolling 30 days)`
            `Block entry of new Features if Ratio < 20%`
            `Alert Tech Lead if trending toward < 20%` </small></small>

### SYMPTOM: Errors explode late and are expensive
- WHY
  - 1. No one warned when it was small 
    - 2. Fear of appearing incompetent or conflictive
      - 3. **"Political correctness" (Psychological Insecurity)** <small><small>
        **DATA:** Psychological Safety explains 43% of the variance in performance *(Google Project Aristotle)* </small></small>
        - STRATEGY 
          - Truth Culture *(Netflix)* <small><small>
            `Post-Mortems "Blameless" (without culprits)`
            `Radical Transparency (Data > Opinions)`
            `Single source of truth (SoT)`
            `Objective opinions, Logic > Emotion`
            **[AGENT - Assistant]** `Anonymize initial incident reports to encourage early reporting`
            `Facilitate weighted anonymous voting by aptitude in technical decisions`
            `Generate automatic post-mortem reports from logs` </small></small>

### SYMPTOM: Inefficiency by Clientelism
- WHY
  - 1. Low levels of internal demand 
    - 2. Culture of "Taking it easy"
      - 3. **Interdependence, cross-incentives** - STRATEGY 
          - Separation of Powers (Checks & Balances) <small><small>
            `Own and opposing indicators per role`
            `Flow: Client -> Demands from PO -> Demands from Tech Lead` 
            **[AGENT - Monitor]** `Red Alert if: Internal KPIs (Green) vs Client KPI (Red)`
            `Detect divergence between internal metrics and external satisfaction`
            `Automatically escalate discrepancies to the higher level` </small></small> 
            
### SYMPTOM: Equity risk 
- WHY
  - 1. Devaluation of assets or cash flow 
    - 2. **Fragility against Local Monetary Policy** - STRATEGY 
        - A. Antifragility and Diversification <small><small>
          **[AGENT - Monitor]** `Macro Monitoring (Inflation/Currency)`
          `Alert if loss of real value > defined threshold`
          `Suggest hedging or diversification strategies` </small></small>

### SYMPTOM: Death by Running Out of Cash
- WHY
  - 1. The company stops because there is no operating cash
    - 2. Burn rate tracked late or not at all (distinct from asset/equity risk)
      - 3. Receivables uncollected; runway never projected forward
        - 4. **No operating-cash early warning** (runway blindness) <small><small>
          **DATA:** Running out of cash is the #2 root cause of failure — 29% of post-mortems *(CB Insights)* </small></small>
          - STRATEGY
            - A. Runway Monitoring (Push alerts, raw numbers) <small><small>
              `Track cash-on-hand, burn rate, projected runway (months)`
              `Distinguish operating liquidity from investment ROE (see principios-operativos.md)`
              **[AGENT - Monitor]** `Alert if projected runway < defined threshold (e.g. 6 months)`
              `Alert on overdue receivables (aging)`
              `Send raw cash figures, not charts (Management by Exception)` </small></small>

### SYMPTOM: Customer Churn (silent revenue leak)
- WHY
  - 1. Revenue erodes without a clear internal cause
    - 2. Internal KPIs look green while customers leave
      - 3. Retention/churn is not measured as a first-class output
        - 4. **Optimizing the factory, ignoring the customer** (external truth missing) <small><small>
          **DATA:** Complements the "Clientelism" symptom — internal-vs-client KPI divergence </small></small>
          - STRATEGY
            - A. Customer as an Output Metric (external SoT) <small><small>
              `Track retention/churn and client satisfaction as core outputs`
              `Client KPI outranks internal KPIs when they diverge (per Checks & Balances)`
              **[AGENT - Monitor]** `Alert on churn spike or retention drop`
              `Red alert if internal KPIs green while client KPI red (reuse divergence rule)`
              **[AGENT - Assistant]** `Summarize churn reasons from channel/support history (RAG)` </small></small>

### SYMPTOM: Incident Without Recovery (continuity)
- WHY
  - 1. An incident (data loss, breach, outage) halts operations
    - 2. Backups/recovery assumed but never exercised
      - 3. No defined recovery point/time; no drill
        - 4. **Continuity assumed, not verified** (BCP/DR gap) <small><small>
          **NOTE:** `Architecture provides R2 + WAL + checkpointing (see FABRIC.md); continuity must be a verified need, not a hope` </small></small>
          - STRATEGY
            - A. Verified Continuity (test the recovery, not the backup) <small><small>
              `Define RPO/RTO per critical resource`
              `Recovery drills as a recurring automated task (Technical Kaizen)`
              `Crash-recovery via local WAL + S3/R2 confirmation (per FABRIC.md)`
              **[AGENT - Monitor]** `Alert if a backup/replication check fails`
              `Alert if a recovery drill has not run within its interval`
              **[AGENT - Guardian]** `Block changes to critical data paths without a verified recovery point` </small></small>