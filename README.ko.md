# jarvis-bench

**로컬 모델 에이전트가 한국 사이트에서 조사해 문서까지 만드는 일을, 진 결과까지 전부 남기며 잰 기록.**

[English](README.md) · 방법: [docs/METHOD.ko.md](docs/METHOD.ko.md) · 결과: [docs/RESULTS.ko.md](docs/RESULTS.ko.md) · 우리 눈금이 틀린 기록: [docs/MISMEASUREMENTS.ko.md](docs/MISMEASUREMENTS.ko.md)

---

## 무엇을 쟀나

JARVIS 는 우리가 만드는 로컬 우선 에이전트다. 이 레포는 그 에이전트가 **「사이트에 가서 보고, 파일로 정리한다」**를
얼마나 해내는지 잰 벤치의 방법·결과·원자료를 담는다. 에이전트 본체 코드는 없다.

```
「사람인에서 여수 회계 채용 공고 3개를 조사해서 문서로 정리해줘. 회사명·고용형태·마감일·공고 URL 을 표로」
```

이런 부탁을 실제 사이트(위시켓·YES24·사람인 …)에 로컬 모델(Gemma 4 26B QAT, RTX 3090)로 돌리고, 결과 파일을 **코드로** 채점했다.

## 한 줄 결과

| | 우리가 고치며 돌린 5과제 (2026-09-13, n=3) | 결과 보기 전에 박아 둔 새 과제 5개 (2026-09-14, n=1) |
|---|---|---|
| 파일 나옴 | 14/15 | 4/5 |
| 표 칸 채움 | 76% | 55% |
| 양식·근거 전부 통과 | 9/14 | 2/4 |
| 읽은 원문에 없는 숫자 | 0 | 0 |

**우리 5과제 점수에는 익숙한 사이트 효과가 섞여 있었다.** 문서를 만드는 쪽의 수정은 처음 보는 과제에서도 통했고,
처음 보는 사이트의 상세 페이지에서 칸(정가·고용형태)을 찾는 쪽이 떨어졌다. 자세한 흐름은 [결과](docs/RESULTS.ko.md).

## 이 레포에서 제일 중요한 것

**점수가 오른 기록보다, 점수가 틀렸던 기록이 많다.** 88% 로 시작한 숫자는 n=2 였고, 그 실패 한 판을 더 돌리자
네 번 중 세 번 실패하는 자리였다. 사흘 동안 찾은 고장의 대부분은 모델이 아니라 **우리 배선과 우리 눈금**이었다 —
[눈금이 틀린 기록](docs/MISMEASUREMENTS.ko.md)에 모았다.

## 들어 있는 것

```
results/   판마다 원자료(_runs.json)와 채점(_grade.json). 이름 = 날짜_표본수_빌드
bench/     multiturn_bench.py  판을 돌리고 장부에서 사슬·파일·근거를 뽑는다
           content_grade.py    표 칸이 시킨 만큼 찼나
           doc_form.py         문서 양식 + 파일 속 숫자·주소가 그 판에서 실제로 읽은 원문에 있나
docs/      방법 · 결과 · 눈금이 틀린 기록
```

벤치 코드는 로컬에서 도는 JARVIS 에 요청을 보내므로 JARVIS 없이는 판을 다시 돌릴 수 없다. 채점기(`content_grade.py`·
`doc_form.py`)는 파일만 있으면 돈다. 숫자 정규화는 공개 라이브러리 [evidence-ledger](https://github.com/KIMDONGJU021211/evidence-ledger) 를 쓴다.

## 안 담은 것

- 에이전트 본체 코드
- 판이 만든 파일 사본 — 사이트에서 옮긴 공고·기사 문장이 들어 있어 저작권 문제가 된다. 채점 결과만 담았다
- 로컬 경로·계정·토큰 — 공개 전에 기계로 검사했다

## 채점기 돌리기

```bash
pip install git+https://github.com/KIMDONGJU021211/evidence-ledger python-docx openpyxl
python bench/content_grade.py <runs.json>
```

## 라이선스

Apache-2.0. 저자는 [AUTHORS](AUTHORS).
