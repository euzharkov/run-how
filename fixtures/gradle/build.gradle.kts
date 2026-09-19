plugins { id("org.springframework.boot") version "3.3.0" apply false }
tasks.register("generateDocs") {
    description = "Generate the API docs"
}
