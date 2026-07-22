import { Link } from "react-router-dom";
import Tracey from "../Tracey";

export default function NotFound() {
  return (
    <div style={{ textAlign: "center", padding: "60px 0" }}>
      <Tracey size={72} expression="thinking" />
      <h1 className="page-title" style={{ marginTop: 16 }}>
        Nothing here.
      </h1>
      <p className="page-sub" style={{ margin: "0 auto 20px" }}>
        This run or page doesn't exist — it may have been rolled back, or the
        link is stale.
      </p>
      <Link to="/" className="btn">
        Back to dashboard
      </Link>
    </div>
  );
}
