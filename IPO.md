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