val scala3Version = "3.7.0"

lazy val root = project
  .in(file("."))
  .settings(
    name := "stream-processing",
    version := "0.1.0-SNAPSHOT",

    scalaVersion := scala3Version,

    libraryDependencies ++= Seq(
      "org.scalameta" %% "munit" % "1.0.0" % Test,
      ("org.apache.spark" %% "spark-sql" % "3.5.5" % "provided").cross(CrossVersion.for3Use2_13),
      //("org.apache.spark" %% "spark-sql-kafka-0-10" % "3.5.5" % "provided").cross(CrossVersion.for3Use2_13),
      ("org.apache.spark" % "spark-streaming" % "3.5.5" % "provided").cross(CrossVersion.for3Use2_13),
      ("org.apache.spark" % "spark-core" % "3.5.5" % "provided").cross(CrossVersion.for3Use2_13),
      ("org.apache.spark" %% "spark-streaming-kafka-0-10" % "3.5.5" % "provided").cross(CrossVersion.for3Use2_13),
      "org.apache.bahir" %% "spark-sql-streaming-mqtt" % "2.4.0-SNAPSHOT"
    ),
    Compile / run := Defaults.runTask(
      Compile / fullClasspath,
      Compile / run / mainClass,
      Compile / run / runner
    ).evaluated
  )
