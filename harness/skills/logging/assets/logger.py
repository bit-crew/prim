import json
import time
import uuid
import traceback
from contextlib import contextmanager
from typing import Dict, Any, Generator

class WideEventLogger:
    """
    Context manager for Accumulating and emitting a Single Wide JSON Log Event.
    Adheres to the Kiro Standard: One execution unit = One structured log upon completion.
    """
    def __init__(self, event_name: str, environment: str = "production", pipeline_name: str = "unnamed_pipeline"):
        self.log_data: Dict[str, Any] = {
            "timestamp": None,
            "event": event_name,
            "status": "success",
            "total_duration_ms": 0.0,
            "context": {
                "pipeline": pipeline_name,
                "run_id": str(uuid.uuid4()),
                "environment": environment
            },
            "metrics": {},
            "steps": {}
        }
        self.start_time: float = 0.0

    def set_metric(self, name: str, value: Any) -> None:
        """Accumulates metrics in memory."""
        self.log_data["metrics"][name] = value

    def set_context(self, name: str, value: Any) -> None:
        """Accumulates pipeline context metadata."""
        self.log_data["context"][name] = value

    @contextmanager
    def step(self, name: str) -> Generator[Dict[str, Any], None, None]:
        """
        Tracks a step's execution time and metadata.
        Yields a dictionary where you can accumulate step-specific details.
        """
        step_start = time.perf_counter()
        step_meta: Dict[str, Any] = {}
        try:
            yield step_meta
            self.log_data["steps"][name] = {
                "duration_ms": round((time.perf_counter() - step_start) * 1000, 2),
                **step_meta
            }
        except Exception as e:
            self.log_data["steps"][name] = {
                "duration_ms": round((time.perf_counter() - step_start) * 1000, 2),
                "error": str(e),
                **step_meta
            }
            raise

    def start(self) -> None:
        self.start_time = time.perf_counter()

    def flush(self, exception: Exception = None) -> None:
        """Calculates total duration and prints the final Wide Event to stdout as JSON."""
        self.log_data["timestamp"] = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
        if self.start_time > 0:
            self.log_data["total_duration_ms"] = round((time.perf_counter() - self.start_time) * 1000, 2)
        
        if exception:
            self.log_data["status"] = "failed"
            self.log_data["error"] = {
                "type": exception.__class__.__name__,
                "message": str(exception),
                "traceback": traceback.format_exc().splitlines()
            }
        
        # Output ONLY one final log
        print(json.dumps(self.log_data))

@contextmanager
def wide_event(event_name: str, environment: str = "production", pipeline_name: str = "unnamed_pipeline") -> Generator[WideEventLogger, None, None]:
    """Helper to run code inside a Wide Event logging scope."""
    logger = WideEventLogger(event_name, environment, pipeline_name)
    logger.start()
    try:
        yield logger
        logger.flush()
    except Exception as e:
        logger.flush(exception=e)
        raise
