import org.apache.spark._
import org.apache.spark.sql._
import org.apache.spark.sql.SparkSession
import org.apache.spark.sql.SparkSession._
import org.apache.spark.sql.functions._
import org.apache.spark.sql.types._

import org.apache.kafka.clients.consumer.ConsumerConfig
import org.apache.kafka.common.serialization.StringDeserializer

import org.apache.spark.SparkConf
import org.apache.spark.streaming._
//import org.apache.spark.streaming.kafka010._
import org.apache.kafka.clients.consumer.ConsumerRecord
//import org.apache.spark.streaming.kafka010.LocationStrategies.PreferConsistent
//import org.apache.spark.streaming.kafka010.ConsumerStrategies.Subscribe
import org.apache.logging.log4j.Level
import org.apache.logging.log4j.core.config.Configurator

object App {
  def main(args: Array[String]): Unit = {
    println(classOf[org.apache.commons.lang3.SystemUtils].getResource("SystemUtils.class"))

    Configurator.setRootLevel(Level.WARN)

    val spark = SparkSession.builder
      .appName("Sensor data processing")
      .master("local[*]")
      .getOrCreate()
    val df = spark.readStream
      .format("kafka")
      .option("kafka.bootstrap.servers", "150.65.230.59:9092")
      .option("subscribe", "i483-sensors-s2510030-BH1750-illumination")
      .load;
    val df_casted = df.select(
      df("timestamp"),
      df("value").cast("string").cast("double")
    );

      df.explain()
      df_casted.explain()
      val res = df_casted.groupBy(
        window(df("timestamp"), "5 minutes", "30 seconds"),
        df_casted("value"))
        .mean("value")
        .orderBy("window")
        //println(res.toString);


    val df_out = df_casted.select(
      df_casted("timestamp"),
      df_casted("value").cast("string")
    );
      val query = df_out.writeStream
        //.outputMode("complete")
        .format("kafka")
        .option("kafka.bootstrap.servers", "150.65.230.59:9092")
        .option("checkpointLocation", "/tmp/i483-spark-checkpoints")
        .option("topic", "i483-sensors-s2510150-analytics-s2510030_BH1750_avg-illumination")
        //.option("truncate", "false")
        .start()


    val co2_data = spark.readStream
      .format("kafka")
      .option("kafka.bootstrap.servers", "150.65.230.59:9092")
      .option("subscribe", "i483-sensors-s2510030-SCD41-co2")
      .load;

      // lag/lead
    val co2_data_rdd = co2_data
        .select(co2_data("value").cast("string").cast("double"))
        .rdd
        .glom()
        .map(ls => ls.sliding(2).flatMap(xs =>
            if (xs(0) < 700 && 700 < xs(1)) {
              Seq("YES")
            } else if (700 < xs(0) && xs(1) < 700) {
              Seq("NO")
            } else {
              Seq[String]()
            }
          ))
    val co2_data_ = spark.createDataset(co2_data_rdd);

    val td_query = co2_data_
        .writeStream
        .format("kafka")
        .option("kafka.bootstrap.servers", "150.65.230.59:9092")
        .option("checkpointLocation", "/tmp/i483-spark-checkpoints")
        .option("topic", "i483-sensors-s2510150-co2_threshold-crossed")
        .start()

      query.awaitTermination()
      query.awaitTermination()

    //val sparkConf = new SparkConf().setAppName("Sensor data processing")
      //.setMaster("local[*]")
    //val ssc = new StreamingContext(sparkConf, Seconds(2))

    //val kafkaParams = Map[String, Object](
      //"bootstrap.servers" -> "150.65.230.59:9092",
      //"key.deserializer" -> classOf[StringDeserializer],
      //"value.deserializer" -> classOf[StringDeserializer],
      //"group.id" -> "KafkaData",
      //"auto.offset.reset" -> "latest",
      //"enable.auto.commit" -> (false: java.lang.Boolean)
    //)

    //val topics = Array("i483-sensors-s2510030-BH1750-illumination")
    //val stream = KafkaUtils.createDirectStream[String, String](
      //ssc,
      //PreferConsistent,
      //Subscribe[String, String](topics, kafkaParams)
    //)

    //stream //.map(record => s"Kafka Data: $record")
      //.groupBy(
        //window($"CreateTime", "5 minutes", "30 seconds"),
        //$"value")
      //.print()
    //ssc.start()
    //ssc.awaitTermination()
  }
}
