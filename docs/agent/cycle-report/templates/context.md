# Cycle Report Context Bundle

## Cycle

- cycle_id:
- cycle_alias:
- cycle_category:
- cycle_category_key:
- cycle_sequence:
- cycle_title:
- status: success | failure | interrupted | blocked
- started_at:
- ended_at:
- close_trigger: orchestra 명시 dispatch
- report_generated_at:

close trigger 는 반드시 `orchestra` 의 close decision 과 명시 dispatch 에서 온다.
polling, idle trace 상태, 멈춘 파일 변경, 시간 경과만으로 cycle end 를 추론하지 않는다.

`cycle_id` 는 canonical 식별자로 유지한다. `cycle_alias` 는 사람이 짧게 부르는
report/index 별칭이며 `category-NNN` 형식이어야 한다. full alias 가 주어진 경우
그 값을 그대로 예약하고 충돌 시 fail-closed 처리한다. category 만 주어진 경우
같은 `cycle_category_key` 의 max sequence + 1 을 예약한 결과를 기록한다.
예약 뒤에는 `trace resolve-alias --alias <cycle_alias>` readback 결과가 현재
`cycle_id` 와 일치하는지 기록한다. alias 예약/readback 누락은 성공처럼 보정하지 않고
`audit.missing_required_inputs` 또는 `audit.warnings` 에 남긴다.

예약 예시는 placeholder 로만 둔다:

- cycle_id: `<cycle-id>`
- cycle_alias: `<category-NNN>`

## User Requests

- user_request_verbatim:
  - text:
  - source_type:
  - source_ref:
  - role:
  - agent_id:
  - hash_sha256:
  - timestamp:
  - order:
- user_request_display_summary_ko:
- legacy_user_request_derived_only:
- result_summary:
- orchestra_instruction: legacy derived field, not verbatim evidence

### Initial Request

-

### Corrections

-

### Approvals

-

### Stop Or Blocked Signals

-

## Planning

### Initial Plan

-

### Final Completion Report

-

### User Response To Final Report

-

## Role Dispatch And Return

원문 지시가 없는 경우 `prompt_verbatim` 을 null 로 두고 `audit.missing_evidence` 에 기록한다.
`prompt_derived_summary_ko` 는 원문이 아니라 표시용 파생 요약이다.

| order | role | agent_id | dispatch_event_id | source_ref | prompt_verbatim_hash_sha256 | prompt_derived_summary_ko |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | planning |  |  |  |  |  |
| 2 | codegen |  |  |  |  |  |
| 3 | review |  |  |  |  |  |
| 4 | test |  |  |  |  |  |
| 5 | analysis |  |  |  |  |  |
| 6 | user-docs |  |  |  |  |  |
| 7 | ai-docs |  |  |  |  |  |
| 8 | cycle-report |  |  |  |  |  |

| order | role | dispatch_summary | return_summary | context_report | accepted_by_orchestra |
| --- | --- | --- | --- | --- | --- |
| 1 | planning |  |  |  |  |
| 2 | codegen |  |  |  |  |
| 3 | review |  |  |  |  |
| 4 | test |  |  |  |  |
| 5 | analysis |  |  |  |  |
| 6 | user-docs |  |  |  |  |
| 7 | ai-docs |  |  |  |  |

## Role Returns

`report.json` 의 `role_returns` top-level 객체로도 반영한다.

| role | return_summary |
| --- | --- |
| planning |  |
| codegen |  |
| review |  |
| test |  |
| analysis |  |
| user-docs |  |
| ai-docs |  |
| cycle-report |  |

## Full Change Records

`changed_files` 와 `files` 는 derived/display 보조 색인이다. 기본 변경 근거는
`report.json.code_changes[]` 의 전체 hunk/line 기록과 `audit.excluded_code_changes[]`
의 제외 사유다.

| path | actor | intent | status |
| --- | --- | --- | --- |
|  |  |  |  |

## Code Changes

`report.json` 의 `code_changes` 배열에 cycle baseline 에서 HEAD 까지의 코드 변경을 구조화해 반영한다.
이 배열은 summary 가 아니라 기본 hunk-level 변경 evidence 다.
이 섹션은 사용자가 코드 변경을 편집기처럼 읽게 하는 표시/보고서 레이어이며, source code 안에 설명용 한국어 주석을 삽입하라는 요구가 아니다.

- diff_baseline_ref:
- diff_head_ref:
- diff_command:
- code_changes_collection_status: collected | partially-collected | skipped

### 수집 규칙

- 기준은 `orchestra` 가 넘긴 cycle baseline 과 현재 HEAD 다.
- baseline 또는 HEAD 가 누락되면 추정하지 말고 `audit.json` 의 `missing_required_inputs` 에 기록한다.
- 신규 파일은 파일 전체를 new-file hunk 로 수집한다. 이때 `old_start` 는 null 또는 0, `old_lines` 는 0 으로 둔다.
- 수정 파일은 unified diff hunk 를 기준으로 `old_start`, `old_lines`, `new_start`, `new_lines`, `lines[]` 를 채운다.
- 삭제 파일은 deleted-file hunk 로 수집한다. 이때 `new_start` 는 null 또는 0, `new_lines` 는 0 으로 둔다.
- rename/copy/mode change 는 `change_kind` 에 반영하고, 실제 content hunk 가 없으면 `hunks` 는 빈 배열로 둘 수 있다.
- 각 hunk 의 `summary_ko` 와 `explanations[]` 는 보고서 레이어 설명이다. 원본 코드 줄 내용은 그대로 두고, 설명은 line/range reference 로 연결한다.

### code_changes 파일 구조

| file_path | language | change_kind | author_roles | summary_ko | raw_diff_ref |
| --- | --- | --- | --- | --- | --- |
|  |  | added \| modified \| deleted \| renamed \| copied \| mode_changed \| unknown |  |  |  |

선택적으로 각 `code_changes[]` 항목에도 report top-level 별칭 필드를 복사해
viewer 가 변경 hunk 옆에 `cycle_alias`, `cycle_category`, `cycle_category_key`,
`cycle_sequence`, `cycle_title` 을 표시할 수 있게 한다.

#### hunk 구조

| file_path | old_start | old_lines | new_start | new_lines | heading | summary_ko |
| --- | --- | --- | --- | --- | --- | --- |
|  |  |  |  |  |  |  |

#### line 구조

| file_path | hunk_heading | kind | old_line | new_line | content |
| --- | --- | --- | --- | --- | --- |
|  |  | context \| add \| remove |  |  |  |

#### explanation 구조

| file_path | hunk_heading | line_ref | range_ref | text_ko |
| --- | --- | --- | --- | --- |
|  |  |  |  |  |

### audit 제외 기록

민감 정보, 대형 파일, 바이너리 파일, 생성 산출물, 라이선스상 전문 표시가 부적절한 파일은 display diff 에 포함하지 않는다.
제외한 경우에도 raw evidence 경로와 제외 사유를 `audit.json` 에 남긴다.

| file_path | exclusion_reason | evidence_ref | audit_json_key |
| --- | --- | --- | --- |
|  | sensitive \| large \| binary \| generated \| license \| unavailable |  | excluded_code_changes |

## Commands And Verification Raw Evidence

- verification_result:

| command | actor | result | evidence |
| --- | --- | --- | --- |
|  |  |  |  |

가능하면 command/result/output 원문은 `command_verbatim`, `result_verbatim`,
`output_verbatim` 또는 `raw.json` 의 test role trace 로 남긴다. 없으면
요약으로 복원하지 말고 `audit.missing_evidence[]` 에 누락을 기록한다.

## Failure Or Blocked Evidence

- failure_point:
- failed_stage:
- failed_role:
- user_visible_symptom:
- root_cause_summary:
- unresolved_error:
- blocked_reason:

## Trace Export And Audit

- development_trace_ref:
- orchestra_trace_ref:
- raw_export_ref:
- audit_ref:
- missing_evidence:
  -
- derived_not_verbatim:
  -
- missing_events:
- inconsistent_events:

trace DB export, file diff raw evidence, 원본 context bundle, 제외 기록은
`raw.json` 또는 `audit.json` 에 보존한다. `index.html` 과 `report.json` 은
display 필드를 압축하거나 재배열할 수 있지만, cycle 을 더 깔끔하게 보이게 하려고
raw evidence 를 고쳐 쓰면 안 된다.

기본 viewer 는 사용자 요청 원문, 역할 지시 원문, 역할 반환 원문, test 명령/결과
원문, 전체 diff hunk, 제외 사유, audit failure, missing/incomplete/fail 상태를
숨김 없이 보여야 한다. 요약 필드는 `derived`, `summary`, `display` 성격을
이름이나 라벨로 드러낸다.

`aliases.json` 은 report root 의 viewer/index projection 이다.
malformed root/version/entry/field/category consistency 는 전체 alias index 실패로 기록한다.
malformed entry 를 제외하고 나머지 alias 만 성공처럼 표시하지 않는다.
by-alias route 실패를 trace DB fallback report 생성으로 보정하지 않는다.

## Derived Summaries

아래 값들은 화면 표시용 파생 요약이며 원문 evidence 로 표시하지 않는다.

- user_request_display_summary_ko:
- orchestra_instruction_summary_ko:
- role_dispatch_summaries_ko:

## Limitations

-

## Next Decisions

-

## Artifact Paths

- index_html:
- report_json:
- raw_json:
- audit_json:
- context_md:
- latest_json:
- aliases_json:
- aliases_json_policy: strict-read | missing | malformed
